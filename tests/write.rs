//! Original writer contract and partial-output regressions.
#![allow(clippy::unwrap_used)]
use std::{
    cell::Cell,
    io::{self, Cursor, Seek, SeekFrom, Write},
    rc::Rc,
    sync::atomic::AtomicBool,
};
use unlace::{
    block::Lacing,
    demux::*,
    seek::{Preparation, SeekEvent, SeekLimits, SeekReader, SeekRequest},
    source::SeekSource,
    write::*,
};
const PRIVATE: [u8; 19] = *b"OpusHead\x01\x01\x38\x01\x80\xbb\0\0\0\0\0";
fn track() -> TrackConfig<'static> {
    TrackConfig {
        number: 1,
        uid: 17,
        codec_id: "A_OPUS",
        codec_private: &PRIVATE,
        media: MediaParameters::Audio(AudioParameters {
            sampling_frequency: 48000.0,
            output_sampling_frequency: 48000.0,
            channels: 1,
            bit_depth: Some(16),
        }),
        codec_delay_ns: 6_500_000,
        seek_preroll_ns: 80_000_000,
        default_duration_ns: Some(20_000_000),
        lacing_allowed: true,
        enabled: true,
    }
}
#[derive(Debug, PartialEq)]
struct Packet {
    data: Vec<u8>,
    track: u64,
    ts: Option<i128>,
    padding: i64,
    duration: Option<i128>,
    lace: (u16, u16),
    key: bool,
    invisible: bool,
    discardable: bool,
}
fn read(bytes: &[u8], number: u64) -> (Vec<Packet>, Completion) {
    let mut d = Demuxer::new(
        bytes,
        AudioSelection::Number(number),
        DemuxLimits::default(),
    )
    .unwrap();
    let mut packets = Vec::new();
    loop {
        match d.poll().unwrap() {
            DemuxEvent::Packet(p) => packets.push(Packet {
                data: p.data.to_vec(),
                track: p.track_number,
                ts: p.timestamp_ns,
                padding: p.block_discard_padding_ns,
                duration: p.block_duration_ns,
                lace: (p.lace_index, p.lace_count),
                key: p.keyframe,
                invisible: p.invisible,
                discardable: p.discardable,
            }),
            DemuxEvent::End(c) => return (packets, c),
            _ => (),
        }
    }
}
#[test]
fn separate_live_and_indexed_receipts_preserve_packets_and_trim() {
    let mut live = LiveWriter::new(Vec::new(), &[track()], WriteConfig::default()).unwrap();
    let mut indexed =
        IndexedWriter::new(Cursor::new(Vec::new()), &[track()], WriteConfig::default()).unwrap();
    for i in 0..4 {
        let mut b = EncodedBlock::new(1, i * 20_000_000 - 6_500_000, &[0xf8, 0xff, 0xfe]);
        if i == 3 {
            b.discard_padding_ns = 3_500_000;
            b.duration_ns = Some(20_000_000);
        }
        live.write_block(b).unwrap();
        indexed.write_block(b).unwrap();
    }
    let l = live.finish().unwrap();
    let f = indexed.finish().unwrap();
    assert_eq!(l.completion, OutputCompletion::Live);
    assert_eq!(f.completion, OutputCompletion::Indexed);
    assert_eq!((l.packets, l.blocks, l.cues), (4, 4, 0));
    assert_eq!((f.packets, f.blocks, f.cues), (4, 4, 1));
    assert!(matches!(
        live.finish(),
        Err(WriteError::State(WriteState::Finished))
    ));
    assert!(matches!(
        indexed.write_block(EncodedBlock::new(1, 100_000_000, b"x")),
        Err(WriteError::State(WriteState::Finished))
    ));
    let lb = live.into_inner();
    let fb = indexed.into_inner().into_inner();
    assert_eq!(l.end_offset, lb.len() as u64);
    assert_eq!(f.end_offset, fb.len() as u64);
    let (lp, lc) = read(&lb, 1);
    let (fp, fc) = read(&fb, 1);
    assert_eq!(lp, fp);
    assert_eq!(lc, Completion::UnknownSizeSegment);
    assert_eq!(fc, Completion::FiniteSegment);
    assert_eq!(fp[0].ts, Some(-6_500_000));
    assert_eq!(fp[3].padding, 3_500_000);
}
#[test]
fn all_laces_late_signed_padding_and_underdetermined_times_round_trip() {
    for mode in [Lacing::Fixed, Lacing::Xiph, Lacing::Ebml] {
        for padding in [-25_000_000, 25_000_000] {
            let frames: &[&[u8]] = if mode == Lacing::Fixed {
                &[b"abc", b"def", b"ghi"]
            } else {
                &[b"a", b"bcdef", b"ghi"]
            };
            let mut w =
                IndexedWriter::new(Cursor::new(Vec::new()), &[track()], WriteConfig::default())
                    .unwrap();
            let mut b = EncodedBlock::new(1, -6_500_000, b"");
            b.frames = Frames::Laced {
                frames,
                lacing: mode,
            };
            b.discard_padding_ns = padding;
            b.duration_ns = Some(60_000_000);
            b.invisible = true;
            w.write_block(b).unwrap();
            w.finish().unwrap();
            let (p, _) = read(&w.into_inner().into_inner(), 1);
            assert_eq!(
                p.iter().map(|p| p.data.as_slice()).collect::<Vec<_>>(),
                frames
            );
            assert_eq!(
                p.iter().map(|p| p.ts).collect::<Vec<_>>(),
                [Some(-6_500_000), Some(13_500_000), Some(33_500_000)]
            );
            assert!(
                p.iter()
                    .all(|p| p.padding == padding && p.duration == Some(60_000_000) && p.invisible)
            );
            assert_eq!(p[2].lace, (2, 3));
        }
    }
    let mut t = track();
    t.default_duration_ns = None;
    let mut w = LiveWriter::new(Vec::new(), &[t], WriteConfig::default()).unwrap();
    let mut b = EncodedBlock::new(1, -6_500_000, b"");
    b.frames = Frames::Laced {
        frames: &[b"a", b"b"],
        lacing: Lacing::Fixed,
    };
    w.write_block(b).unwrap();
    w.finish().unwrap();
    let (p, _) = read(&w.into_inner(), 1);
    assert_eq!(p[1].ts, None);
}
#[test]
fn preflight_errors_do_not_change_output_or_poison_valid_following_blocks() {
    let mut w =
        IndexedWriter::new(Cursor::new(Vec::new()), &[track()], WriteConfig::default()).unwrap();
    let position = w.position();
    let mut bad = EncodedBlock::new(1, -6_499_999, b"data");
    assert!(matches!(w.write_block(bad), Err(WriteError::Invalid(_))));
    assert_eq!(w.position(), position);
    bad.timestamp_ns = -6_500_000;
    bad.duration_ns = Some(1);
    assert!(w.write_block(bad).is_err());
    bad.duration_ns = Some(20_000_000);
    bad.discard_padding_ns = i64::MIN;
    assert!(w.write_block(bad).is_err());
    bad.discard_padding_ns = 0;
    bad.track_number = 2;
    assert!(w.write_block(bad).is_err());
    assert_eq!(w.state(), WriteState::Writing);
    assert_eq!(w.position(), position);
    w.write_block(EncodedBlock::new(1, -6_500_000, b"good"))
        .unwrap();
    let position = w.position();
    assert!(
        w.write_block(EncodedBlock::new(1, -26_500_000, b"old"))
            .is_err()
    );
    assert_eq!(w.position(), position);
    w.finish().unwrap();
    assert_eq!(read(&w.into_inner().into_inner(), 1).0[0].data, b"good");
}
#[test]
fn laced_none_and_single_frame_laces_are_not_silently_unlaced() {
    for lacing in [Lacing::None, Lacing::Fixed, Lacing::Xiph, Lacing::Ebml] {
        let mut w = LiveWriter::new(Vec::new(), &[track()], WriteConfig::default()).unwrap();
        let at = w.position();
        let mut b = EncodedBlock::new(1, -6_500_000, b"");
        b.frames = Frames::Laced {
            frames: &[b"a"],
            lacing,
        };
        assert!(w.write_block(b).is_err(), "{lacing:?}");
        assert_eq!(w.position(), at);
        assert_eq!(w.state(), WriteState::Writing);
    }
}
#[test]
fn metadata_is_checked_before_output_and_identity_is_retained() {
    for kind in 0..5 {
        let mut sink = Vec::new();
        let mut t = track();
        match kind {
            0 => t.number = 0,
            1 => t.uid = 0,
            2 => t.codec_delay_ns = 1,
            3 => t.codec_private = b"bad",
            _ => t.codec_id = "A_UNKNOWN",
        };
        assert!(LiveWriter::new(&mut sink, &[t], WriteConfig::default()).is_err());
        assert!(sink.is_empty());
    }
    let mut sink = Vec::new();
    assert!(LiveWriter::new(&mut sink, &[track(), track()], WriteConfig::default()).is_err());
    assert!(sink.is_empty());
    let mut t = track();
    t.number = (1 << 56) - 1;
    t.uid = u64::MAX;
    let mut w = LiveWriter::new(Vec::new(), &[t], WriteConfig::default()).unwrap();
    w.write_block(EncodedBlock::new(t.number, -6_500_000, b"wide"))
        .unwrap();
    w.finish().unwrap();
    let bytes = w.into_inner();
    let mut d = Demuxer::new(
        bytes.as_slice(),
        AudioSelection::Number(t.number),
        DemuxLimits::default(),
    )
    .unwrap();
    loop {
        match d.poll().unwrap() {
            DemuxEvent::TracksReady => break,
            DemuxEvent::Yield => (),
            event => panic!("expected TracksReady, got {event:?}"),
        }
    }
    let actual = &d.tracks()[0];
    assert_eq!(actual.uid, u64::MAX);
    assert_eq!(actual.codec_private, PRIVATE);
    assert_eq!(actual.default_duration_ns, t.default_duration_ns);
    assert_eq!(actual.audio.unwrap().bit_depth, Some(16));
    assert_eq!(read(&bytes, t.number).0[0].track, t.number);
}
#[test]
fn index_caps_refuse_before_block_output_and_finish_valid_prior_work() {
    let cfg = WriteConfig {
        cue_interval_ns: 0,
        limits: WriteLimits {
            max_cues: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut w = IndexedWriter::new(Cursor::new(Vec::new()), &[track()], cfg).unwrap();
    w.write_block(EncodedBlock::new(1, -6_500_000, b"first"))
        .unwrap();
    let position = w.position();
    assert!(matches!(
        w.write_block(EncodedBlock::new(1, 13_500_000, b"second")),
        Err(WriteError::Limit("Cues"))
    ));
    assert_eq!(w.position(), position);
    assert_eq!(w.state(), WriteState::Writing);
    w.finish().unwrap();
    assert_eq!(read(&w.into_inner().into_inner(), 1).0.len(), 1);
}
#[test]
fn final_index_positions_are_verified_by_the_separate_seek_reader() {
    let cfg = WriteConfig {
        cue_interval_ns: 20_000_000,
        ..Default::default()
    };
    let mut w = IndexedWriter::new(Cursor::new(Vec::new()), &[track()], cfg).unwrap();
    for i in 0..10 {
        w.write_block(EncodedBlock::new(1, i * 20_000_000 - 6_500_000, &[i as u8]))
            .unwrap();
    }
    let receipt = w.finish().unwrap();
    assert_eq!(receipt.cues, 10);
    let bytes = w.into_inner().into_inner();
    let mut s = SeekReader::new(
        SeekSource::new(Cursor::new(bytes), 1),
        AudioSelection::First,
        DemuxLimits::default(),
        SeekLimits::default(),
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    while s.prepare(&cancel).unwrap() != Preparation::Ready {}
    for target in [100_000_000, 150_000_000, 0, 80_000_000] {
        let result = s
            .seek(
                SeekRequest {
                    timestamp_ns: target,
                    preroll_ns: 0,
                },
                &cancel,
            )
            .unwrap();
        assert!(result.actual_timestamp_ns <= target);
        let mut saw = false;
        loop {
            match s.poll(&cancel).unwrap() {
                SeekEvent::Packet(p) => {
                    assert_eq!(p.data.len(), 1);
                    saw = true;
                }
                SeekEvent::End(_) => break,
                SeekEvent::Yield => (),
            }
        }
        assert!(saw);
    }
}
#[test]
fn disabled_and_enabled_tracks_keep_number_metadata_and_order() {
    let mut disabled = track();
    disabled.enabled = false;
    let mut selected = track();
    selected.number = 2;
    selected.uid = 18;
    let mut w = IndexedWriter::new(
        Cursor::new(Vec::new()),
        &[disabled, selected],
        WriteConfig::default(),
    )
    .unwrap();
    w.write_block(EncodedBlock::new(1, -6_500_000, b"disabled"))
        .unwrap();
    w.write_block(EncodedBlock::new(2, -6_500_000, b"enabled"))
        .unwrap();
    w.finish().unwrap();
    let bytes = w.into_inner().into_inner();
    assert_eq!(read(&bytes, 2).0[0].data, b"enabled");
    let mut d = Demuxer::new(
        bytes.as_slice(),
        AudioSelection::First,
        DemuxLimits::default(),
    )
    .unwrap();
    loop {
        match d.poll().unwrap() {
            DemuxEvent::TracksReady => break,
            DemuxEvent::Yield => (),
            event => panic!("expected TracksReady, got {event:?}"),
        }
    }
    assert!(!d.tracks()[0].enabled);
    assert!(d.tracks()[1].enabled);
}
struct Fault {
    cursor: Cursor<Vec<u8>>,
    written: usize,
    fail_at: usize,
    chunk: usize,
    interrupts: usize,
    calls: usize,
    flushes: usize,
    fail_flush: usize,
    bad_seek: Rc<Cell<bool>>,
}
impl Fault {
    fn new(fail_at: usize) -> Self {
        Self {
            cursor: Cursor::new(Vec::new()),
            written: 0,
            fail_at,
            chunk: usize::MAX,
            interrupts: 0,
            calls: 0,
            flushes: 0,
            fail_flush: 0,
            bad_seek: Rc::new(Cell::new(false)),
        }
    }
}
impl Write for Fault {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.interrupts > 0 {
            self.interrupts -= 1;
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.written >= self.fail_at {
            return Err(io::ErrorKind::StorageFull.into());
        }
        let n = data.len().min(self.chunk).min(self.fail_at - self.written);
        let n = self.cursor.write(&data[..n])?;
        self.written += n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.calls += 1;
        self.flushes += 1;
        if self.flushes == self.fail_flush {
            Err(io::ErrorKind::StorageFull.into())
        } else {
            Ok(())
        }
    }
}
impl Seek for Fault {
    fn seek(&mut self, where_: SeekFrom) -> io::Result<u64> {
        self.calls += 1;
        let n = self.cursor.seek(where_)?;
        Ok(n + u64::from(self.bad_seek.get()))
    }
}
fn produce(fault: &mut Fault) -> Result<WriteReceipt, WriteError> {
    let mut w = IndexedWriter::new(fault, &[track()], WriteConfig::default())?;
    w.write_block(EncodedBlock::new(1, -6_500_000, b"one"))?;
    w.write_block(EncodedBlock::new(1, 13_500_000, b"two"))?;
    w.finish()
}
#[test]
fn every_partial_write_failure_has_no_success_receipt() {
    let mut normal = Fault::new(usize::MAX);
    let expected = produce(&mut normal).unwrap();
    let writes = normal.written;
    assert_eq!(expected.completion, OutputCompletion::Indexed);
    for at in 0..writes {
        let mut failed = Fault::new(at);
        let error = produce(&mut failed).unwrap_err();
        assert!(matches!(error, WriteError::Io { .. }), "at {at}: {error}");
        assert_eq!(failed.written, at);
    }
    let mut tiny = Fault::new(usize::MAX);
    tiny.chunk = 1;
    tiny.interrupts = 7;
    let got = produce(&mut tiny).unwrap();
    assert_eq!(got, expected);
    assert_eq!(tiny.cursor.into_inner(), normal.cursor.into_inner());
}
#[test]
fn failures_poison_writer_and_bad_seek_positions_are_detected() {
    let mut fault = Fault::new(usize::MAX);
    let flag = fault.bad_seek.clone();
    let mut w = IndexedWriter::new(&mut fault, &[track()], WriteConfig::default()).unwrap();
    w.write_block(EncodedBlock::new(1, -6_500_000, b"one"))
        .unwrap();
    flag.set(true);
    assert!(matches!(w.finish(), Err(WriteError::Position { .. })));
    assert_eq!(w.state(), WriteState::Failed);
    let position = w.position();
    assert!(matches!(
        w.finish(),
        Err(WriteError::State(WriteState::Failed))
    ));
    assert_eq!(w.position(), position);
}
#[test]
fn flush_failure_never_returns_receipt_even_when_header_is_already_finite() {
    for fail_flush in [1, 2] {
        let mut fault = Fault::new(usize::MAX);
        fault.fail_flush = fail_flush;
        let mut w = IndexedWriter::new(&mut fault, &[track()], WriteConfig::default()).unwrap();
        w.write_block(EncodedBlock::new(1, -6_500_000, b"one"))
            .unwrap();
        assert!(matches!(
            w.finish(),
            Err(WriteError::Io {
                stage: WriteStage::Flush,
                ..
            })
        ));
        assert_eq!(w.state(), WriteState::Failed);
        let bytes = w.into_inner().cursor.get_ref();
        let (_, completion) = read(bytes, 1);
        assert_eq!(
            completion,
            if fail_flush == 1 {
                Completion::UnknownSizeSegment
            } else {
                Completion::FiniteSegment
            }
        );
    }
}
#[test]
fn no_drop_or_extraction_path_claims_indexed_finalization() {
    let mut output = Cursor::new(Vec::new());
    {
        let mut w = IndexedWriter::new(&mut output, &[track()], WriteConfig::default()).unwrap();
        w.write_block(EncodedBlock::new(1, -6_500_000, b"one"))
            .unwrap();
    }
    assert_eq!(read(output.get_ref(), 1).1, Completion::UnknownSizeSegment);
    let nonempty = Cursor::new(vec![1, 2]);
    assert!(matches!(
        IndexedWriter::new(nonempty, &[track()], WriteConfig::default()),
        Err(WriteError::NonemptySink)
    ));
}
#[test]
fn constructor_seeks_obey_the_public_operation_io_call_budget() {
    let mut config = WriteConfig::default();
    config.limits.max_io_calls = 1;
    let mut sink = Fault::new(usize::MAX);
    assert!(IndexedWriter::new(&mut sink, &[track()], config).is_err());
    assert!(sink.calls <= 1, "{} calls", sink.calls);
}
#[test]
fn live_output_does_not_require_seek_or_retain_a_recording_history() {
    struct Count(u64);
    impl Write for Count {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.0 += b.len() as u64;
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut w = LiveWriter::new(Count(0), &[track()], WriteConfig::default()).unwrap();
    for second in 0..21601 {
        w.write_block(EncodedBlock::new(
            1,
            second * 1_000_000_000 - 6_500_000,
            b"x",
        ))
        .unwrap();
    }
    let receipt = w.finish().unwrap();
    assert_eq!(receipt.blocks, 21601);
    assert_eq!(receipt.cues, 0);
    assert_eq!(receipt.end_offset, w.into_inner().0);
}

#[test]
fn metadata_work_limit_precedes_codec_scanning_and_validation() {
    let private = [0u8; 512];
    let mut t = track();
    t.codec_private = &private;
    let config = WriteConfig {
        limits: WriteLimits {
            max_metadata_size: 256,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut sink = Vec::new();
    assert!(matches!(
        LiveWriter::new(&mut sink, &[t], config),
        Err(WriteError::Limit(_))
    ));
    assert!(sink.is_empty());
}
#[test]
fn wide_ebml_lace_differences_empty_frames_and_all_256_slots_round_trip() {
    for sizes in [
        vec![0, 127, 63, 128, 16384, 0],
        vec![1; 256],
        vec![64, 0, 63, 127, 128, 1],
    ] {
        let data: Vec<Vec<u8>> = sizes
            .iter()
            .enumerate()
            .map(|(i, &n)| vec![i as u8; n])
            .collect();
        let frames: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
        for lacing in [Lacing::Xiph, Lacing::Ebml] {
            let mut w = LiveWriter::new(Vec::new(), &[track()], WriteConfig::default()).unwrap();
            let mut b = EncodedBlock::new(1, -6_500_000, b"");
            b.frames = Frames::Laced {
                frames: &frames,
                lacing,
            };
            w.write_block(b).unwrap();
            w.finish().unwrap();
            let (p, _) = read(&w.into_inner(), 1);
            assert_eq!(p.len(), data.len());
            for (a, b) in p.iter().zip(&data) {
                assert_eq!(&a.data, b);
            }
        }
    }
}
#[test]
fn block_io_failure_poison_is_distinct_from_preflight_refusal() {
    let baseline = LiveWriter::new(Vec::new(), &[track()], WriteConfig::default()).unwrap();
    let at = baseline.position();
    let mut sink = Fault::new(at as usize + 3);
    let mut w = LiveWriter::new(&mut sink, &[track()], WriteConfig::default()).unwrap();
    assert!(matches!(
        w.write_block(EncodedBlock::new(1, -6_500_000, b"partial")),
        Err(WriteError::Io {
            stage: WriteStage::Block,
            ..
        })
    ));
    assert_eq!(w.state(), WriteState::Failed);
    assert_eq!(w.position(), at + 3);
    assert!(matches!(
        w.write_block(EncodedBlock::new(1, -6_500_000, b"retry")),
        Err(WriteError::State(WriteState::Failed))
    ));
    assert!(matches!(
        w.finish(),
        Err(WriteError::State(WriteState::Failed))
    ));
}
#[test]
fn reference_and_simpleblock_flags_are_preserved_without_inventing_keyframes() {
    let mut w = LiveWriter::new(Vec::new(), &[track()], WriteConfig::default()).unwrap();
    let mut b = EncodedBlock::new(1, -6_500_000, b"nonkey");
    b.keyframe = false;
    b.invisible = true;
    b.discardable = true;
    w.write_block(b).unwrap();
    b.timestamp_ns += 20_000_000;
    b.discardable = false;
    b.duration_ns = Some(20_000_000);
    w.write_block(b).unwrap();
    w.finish().unwrap();
    let (p, _) = read(&w.into_inner(), 1);
    assert!(!p[0].key && p[0].invisible && p[0].discardable);
    assert!(!p[1].key && p[1].invisible && !p[1].discardable);
}

#[test]
fn exact_index_size_boundary_and_write_work_refusal_are_observable() {
    for bytes in [62, 63] {
        let config = WriteConfig {
            limits: WriteLimits {
                max_index_size: bytes,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut w = IndexedWriter::new(Cursor::new(Vec::new()), &[track()], config).unwrap();
        let at = w.position();
        let result = w.write_block(EncodedBlock::new(1, -6_500_000, b"cue"));
        if bytes == 62 {
            assert!(matches!(result, Err(WriteError::Limit("Cues"))));
            assert_eq!(w.position(), at);
            assert_eq!(w.state(), WriteState::Writing);
        } else {
            result.unwrap();
            assert_eq!(w.finish().unwrap().cues, 1);
        }
    }
    let config = WriteConfig {
        limits: WriteLimits {
            max_io_calls: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut w = LiveWriter::new(Vec::new(), &[track()], config).unwrap();
    assert!(matches!(
        w.write_block(EncodedBlock::new(1, -6_500_000, b"bounded")),
        Err(WriteError::Limit("I/O calls"))
    ));
    assert_eq!(w.state(), WriteState::Failed);
}
#[test]
fn zero_writes_invalid_provider_counts_and_wouldblock_are_terminal() {
    struct Broken(u8);
    impl Write for Broken {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            match self.0 {
                0 => Ok(0),
                1 => Ok(b.len() + 1),
                _ => Err(io::ErrorKind::WouldBlock.into()),
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    for kind in 0..3 {
        let error = match LiveWriter::new(Broken(kind), &[track()], WriteConfig::default()) {
            Err(e) => e,
            Ok(_) => panic!("broken writer accepted"),
        };
        assert!(matches!(
            error,
            WriteError::Io {
                stage: WriteStage::Header,
                ..
            }
        ));
    }
}

#[test]
fn unsupported_opus_initialization_retains_its_distinct_typed_cause() {
    use unlace::opus::{OpusError, OpusErrorKind};
    for (position, value, kind) in [
        (8, 16, OpusErrorKind::UnsupportedVersion),
        (18, 2, OpusErrorKind::UnsupportedMapping),
    ] {
        let mut private = PRIVATE;
        private[position] = value;
        let mut t = track();
        t.codec_private = &private;
        let error = LiveWriter::new(Vec::new(), &[t], WriteConfig::default())
            .err()
            .unwrap();
        assert_eq!(
            std::error::Error::source(&error)
                .and_then(|e| e.downcast_ref::<OpusError>())
                .map(|e| e.kind),
            Some(kind)
        );
    }
}

#[test]
fn valid_inputs_over_policy_limits_are_distinct_from_invalid_representation() {
    let mut second = track();
    second.number = 2;
    second.uid = 18;
    let config = WriteConfig {
        limits: WriteLimits {
            max_tracks: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(matches!(
        LiveWriter::new(Vec::new(), &[track(), second], config),
        Err(WriteError::Limit(_))
    ));
    let mut w = LiveWriter::new(Vec::new(), &[track()], WriteConfig::default()).unwrap();
    let mut b = EncodedBlock::new(1, -6_500_000, b"");
    b.frames = Frames::Laced {
        frames: &[],
        lacing: Lacing::Fixed,
    };
    assert!(matches!(w.write_block(b), Err(WriteError::Invalid(_))));
    let config = WriteConfig {
        limits: WriteLimits {
            block: unlace::block::BlockLimits {
                max_frames: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let mut w = LiveWriter::new(Vec::new(), &[track()], config).unwrap();
    b.frames = Frames::Laced {
        frames: &[b"a", b"b"],
        lacing: Lacing::Fixed,
    };
    assert!(matches!(w.write_block(b), Err(WriteError::Limit(_))));
}
