//! Original indexed fixtures and sequential/seek suffix equivalence.
#![allow(clippy::unwrap_used)]
use std::io::Cursor;
use std::sync::atomic::AtomicBool;
use unlace::demux::{AudioSelection, DemuxEvent, DemuxLimits, Demuxer};
use unlace::seek::{
    Fallback, Preparation, SeekError, SeekEvent, SeekLimits, SeekReader, SeekRequest,
};
use unlace::source::{ReadAt, SeekSource};

fn element(id: u32, data: &[u8]) -> Vec<u8> {
    let id = id.to_be_bytes();
    let first = id.iter().position(|&b| b != 0).unwrap();
    let mut out = id[first..].to_vec();
    let width = (1..=8)
        .find(|w| (data.len() as u64) < (1u64 << (7 * w)) - 1)
        .unwrap();
    let size = data.len() as u64 | (1u64 << (7 * width));
    out.extend_from_slice(&size.to_be_bytes()[8 - width..]);
    out.extend_from_slice(data);
    out
}
fn number(id: u32, value: u64) -> Vec<u8> {
    element(id, &value.to_be_bytes())
}
fn master(id: u32, data: &[u8], unknown: bool) -> Vec<u8> {
    if !unknown {
        return element(id, data);
    }
    let mut out = id.to_be_bytes().to_vec();
    out.push(255);
    out.extend_from_slice(data);
    out
}
fn track(n: u64, kind: u64) -> Vec<u8> {
    let mut out = number(0xd7, n);
    out.extend(number(0x73c5, n + 10));
    out.extend(number(0x83, kind));
    out.extend(element(0x86, if kind == 2 { b"A_TEST" } else { b"V_TEST" }));
    if kind == 2 {
        out.extend(number(0x56aa, 7));
        out.extend(number(0x56bb, 30_000));
        out.extend(number(0x23e383, 20_000));
        let mut audio = element(0xb5, &48000f64.to_be_bytes());
        audio.extend(number(0x9f, 1));
        out.extend(element(0xe1, &audio));
    }
    element(0xae, &out)
}
fn block(track: u8, time: i16, flags: u8, bytes: &[u8]) -> Vec<u8> {
    block_width(track, time, flags, bytes, 1)
}
fn block_width(track: u8, time: i16, flags: u8, bytes: &[u8], width: usize) -> Vec<u8> {
    let encoded = u64::from(track) | (1u64 << (width * 7));
    let mut out = encoded.to_be_bytes()[8 - width..].to_vec();
    out.extend(time.to_be_bytes());
    out.push(flags);
    out.extend_from_slice(bytes);
    out
}
struct Fixture {
    bytes: Vec<u8>,
    clusters: Vec<u64>,
    cue_positions: Vec<u64>,
    fake: u64,
    relative: u64,
}
fn fixture(unknown: bool, cue_track: Option<u64>) -> Fixture {
    fixture_width(unknown, cue_track, 1)
}
fn fixture_width(unknown: bool, cue_track: Option<u64>, width: usize) -> Fixture {
    fixture_options(unknown, cue_track, width, 16_384)
}
fn fixture_options(
    unknown: bool,
    cue_track: Option<u64>,
    width: usize,
    payload_size: usize,
) -> Fixture {
    let mut ebml = element(0x4282, b"webm");
    ebml.extend(number(0x4287, 4));
    ebml.extend(number(0x4285, 2));
    let ebml = element(0x1a45dfa3, &ebml);
    let mut info = element(0x4d80, b"synthetic");
    info.extend(element(0x5741, b"independent test encoder"));
    info.extend(number(0x2ad7b1, 1000));
    let mut segment = element(0x1549a966, &info);
    let mut tracks = track(1, 1);
    tracks.extend(track(2, 2));
    tracks.extend(track(3, 2));
    segment.extend(element(0x1654ae6b, &tracks));
    let mut relative_clusters = Vec::new();
    let mut fake = 0;
    let mut audio_relative = 0;
    for i in 1..=3u64 {
        relative_clusters.push(segment.len() as u64);
        let mut cluster = number(0xe7, i * 100);
        let mut video = vec![77; payload_size];
        video[20..26].copy_from_slice(&[0x1f, 0x43, 0xb6, 0x75, 0x80, 0]);
        let video_header_len = element(0xa3, &block(1, 0, 0x80, &video)).len() - video.len();
        let fake_in_cluster = cluster.len() + video_header_len + 20;
        cluster.extend(element(0xa3, &block(1, 0, 0x80, &video)));
        audio_relative = cluster.len() as u64;
        cluster.extend(element(
            0xa3,
            &block_width(2, 0, 0x80, &[i as u8, 10], width),
        ));
        cluster.extend(element(0xa3, &block(3, 0, 0x80, &[99])));
        let mut group = element(
            0xa1,
            &block_width(2, 20, 4, &[1, i as u8, 20, i as u8, 30], width),
        );
        group.extend(number(0x9b, 40));
        group.extend(element(0x75a2, &[9]));
        cluster.extend(element(0xa0, &group));
        let framed = master(0x1f43b675, &cluster, unknown);
        if i == 2 {
            fake = segment.len() as u64
                + (framed.len() - cluster.len()) as u64
                + fake_in_cluster as u64;
        }
        segment.extend(framed);
    }
    let cues_offset = segment.len() as u64;
    if let Some(track) = cue_track {
        let mut cues = Vec::new();
        for (i, &position) in relative_clusters.iter().enumerate() {
            let mut positions = number(0xf7, track);
            positions.extend(number(0xf1, position));
            positions.extend(number(0xf0, audio_relative));
            let mut point = element(0xb7, &positions);
            point.extend(number(0xb3, (i as u64 + 1) * 100));
            cues.extend(element(0xbb, &point));
        }
        segment.extend(element(0x1c53bb6b, &cues));
        let mut seek = element(0x53ab, &[0x1c, 0x53, 0xbb, 0x6b]);
        seek.extend(number(0x53ac, cues_offset));
        segment.extend(element(0x114d9b74, &element(0x4dbb, &seek)));
    }
    let segment_len = segment.len();
    let segment = master(0x18538067, &segment, unknown);
    let data_offset = (ebml.len() + segment.len() - segment_len) as u64;
    let mut bytes = ebml;
    bytes.extend(segment);
    Fixture {
        bytes,
        clusters: relative_clusters.iter().map(|&p| p + data_offset).collect(),
        cue_positions: relative_clusters,
        fake,
        relative: audio_relative,
    }
}
fn change(bytes: &mut [u8], id: u32, old: u64, new: u64) {
    let from = number(id, old);
    let to = number(id, new);
    let start = bytes.windows(from.len()).position(|w| w == from).unwrap();
    bytes[start..start + to.len()].copy_from_slice(&to);
}
fn reader(bytes: Vec<u8>) -> SeekReader<SeekSource<Cursor<Vec<u8>>>> {
    SeekReader::new(
        SeekSource::new(Cursor::new(bytes), 1),
        AudioSelection::Number(2),
        DemuxLimits::default(),
        SeekLimits::default(),
    )
    .unwrap()
}
fn prepare<S: ReadAt>(reader: &mut SeekReader<S>, cancel: &AtomicBool) -> Result<(), SeekError> {
    for _ in 0..100_000 {
        if reader.prepare(cancel)? == Preparation::Ready {
            return Ok(());
        }
    }
    panic!("preparation did not finish")
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Packet {
    bytes: Vec<u8>,
    timestamp: Option<i128>,
    raw: i128,
    track: u64,
    offset: u64,
    lace: (u16, u16),
    padding: i64,
    duration: Option<i128>,
}
fn packet(p: unlace::demux::Packet<'_>) -> Packet {
    Packet {
        bytes: p.data.to_vec(),
        timestamp: p.timestamp_ns,
        raw: p.block_timestamp_ns,
        track: p.track_number,
        offset: p.block_offset,
        lace: (p.lace_index, p.lace_count),
        padding: p.block_discard_padding_ns,
        duration: p.block_duration_ns,
    }
}
fn sequential(bytes: &[u8]) -> Vec<Packet> {
    let mut demux = Demuxer::new(bytes, AudioSelection::Number(2), DemuxLimits::default()).unwrap();
    let mut out = Vec::new();
    loop {
        match demux.poll().unwrap() {
            DemuxEvent::Packet(p) => out.push(packet(p)),
            DemuxEvent::End(_) => return out,
            _ => (),
        }
    }
}
fn suffix<S: ReadAt>(reader: &mut SeekReader<S>, cancel: &AtomicBool, from: u64) -> Vec<Packet> {
    let mut out = Vec::new();
    for _ in 0..100_000 {
        match reader.poll(cancel).unwrap() {
            SeekEvent::Packet(p) => out.push(packet(p)),
            SeekEvent::Yield => (),
            SeekEvent::End(end) => {
                assert_eq!(end.from_cluster_offset, from);
                return out;
            }
        }
    }
    panic!("suffix did not finish")
}
#[test]
fn indexed_forward_backward_seeks_match_every_sequential_suffix_packet() {
    let cancel = AtomicBool::new(false);
    for unknown in [false, true] {
        let f = fixture(unknown, Some(2));
        let expected = sequential(&f.bytes);
        let mut reader = reader(f.bytes);
        prepare(&mut reader, &cancel).unwrap();
        assert_eq!(reader.tracks().len(), 3);
        for index in [2, 0, 1, 2, 1, 0] {
            let requested = (index as i128 + 1) * 100_000 - 7 + 50_000;
            let result = reader
                .seek(
                    SeekRequest {
                        timestamp_ns: requested,
                        preroll_ns: 50_000,
                    },
                    &cancel,
                )
                .unwrap();
            assert_eq!(result.cluster_offset, f.clusters[index]);
            assert_eq!(result.track_number, 2);
            assert_eq!(result.cue_time_ticks, Some((index as u64 + 1) * 100));
            assert_eq!(result.fallback, None);
            assert_eq!(
                result.actual_timestamp_ns,
                (index as i128 + 1) * 100_000 - 7
            );
            assert_eq!(result.required_preroll_ns, 50_000);
            assert_eq!(result.declared_preroll_ns, 30_000);
            assert!(reader.read_report().bytes < 1000);
            assert_eq!(
                suffix(&mut reader, &cancel, result.cluster_offset),
                expected
                    .iter()
                    .filter(|p| p.offset >= result.cluster_offset)
                    .cloned()
                    .collect::<Vec<_>>()
            );
        }
    }
}
#[test]
fn missing_or_other_track_cues_report_header_scan_fallback() {
    let cancel = AtomicBool::new(false);
    for (cue_track, fallback) in [
        (None, Fallback::MissingCues),
        (Some(3), Fallback::NoTrackCues),
    ] {
        let f = fixture(true, cue_track);
        let mut reader = reader(f.bytes);
        prepare(&mut reader, &cancel).unwrap();
        let result = reader
            .seek(
                SeekRequest {
                    timestamp_ns: 270_000,
                    preroll_ns: 0,
                },
                &cancel,
            )
            .unwrap();
        assert_eq!(result.fallback, Some(fallback));
        assert_eq!(result.cluster_offset, f.clusters[1]);
        assert_eq!(result.required_preroll_ns, 30_000);
        assert_eq!(result.cue_time_ticks, None);
        assert!(reader.read_report().bytes < 1000);
    }
}
#[test]
fn forged_cluster_offsets_cannot_target_matching_bytes_in_payload() {
    let cancel = AtomicBool::new(false);
    for target in [0, 1, u64::MAX] {
        let mut f = fixture(false, Some(2));
        let new = if target == 0 {
            f.fake
        } else if target == 1 {
            f.cue_positions[1] + 1
        } else {
            target
        };
        change(&mut f.bytes, 0xf1, f.cue_positions[1], new);
        let mut reader = reader(f.bytes);
        assert!(matches!(
            prepare(&mut reader, &cancel),
            Err(SeekError::Invalid { .. })
        ));
        assert!(matches!(reader.prepare(&cancel), Err(SeekError::Failed)));
    }
}
#[test]
fn used_cue_requires_actual_selected_track_time_and_relative_boundary() {
    let cancel = AtomicBool::new(false);
    for kind in 0..4 {
        let mut f = fixture(false, Some(2));
        match kind {
            0 => change(&mut f.bytes, 0xf0, f.relative, f.relative + 1),
            1 => change(&mut f.bytes, 0xb3, 100, 99),
            2 => change(&mut f.bytes, 0xf0, f.relative, 10),
            _ => change(&mut f.bytes, 0xf0, f.relative, u64::MAX),
        }
        let mut reader = reader(f.bytes);
        prepare(&mut reader, &cancel).unwrap();
        assert!(matches!(
            reader.seek(
                SeekRequest {
                    timestamp_ns: 180_000,
                    preroll_ns: 0
                },
                &cancel
            ),
            Err(SeekError::Invalid { .. })
        ));
    }
}
#[test]
fn unknown_cue_track_is_rejected_before_index_ready() {
    let cancel = AtomicBool::new(false);
    let f = fixture(false, Some(99));
    let mut reader = reader(f.bytes);
    assert!(matches!(
        prepare(&mut reader, &cancel),
        Err(SeekError::Invalid { .. })
    ));
}

#[test]
fn seek_prefix_supports_all_track_vint_widths_including_eight() {
    let cancel = AtomicBool::new(false);
    for width in 1..=8 {
        let f = fixture_width(false, Some(2), width);
        let mut reader = reader(f.bytes);
        prepare(&mut reader, &cancel).unwrap();
        let result = reader
            .seek(
                SeekRequest {
                    timestamp_ns: 180_000,
                    preroll_ns: 0,
                },
                &cancel,
            )
            .unwrap();
        assert_eq!(result.track_number, 2);
        assert_eq!(result.cluster_offset, f.clusters[0]);
    }
}

#[test]
fn invalid_request_overflow_does_not_poison_a_prepared_source() {
    let cancel = AtomicBool::new(false);
    let f = fixture(false, Some(2));
    let mut reader = reader(f.bytes);
    prepare(&mut reader, &cancel).unwrap();
    assert!(
        reader
            .seek(
                SeekRequest {
                    timestamp_ns: i128::MIN,
                    preroll_ns: 1
                },
                &cancel
            )
            .is_err()
    );
    assert!(
        reader
            .seek(
                SeekRequest {
                    timestamp_ns: 180_000,
                    preroll_ns: 0
                },
                &cancel
            )
            .is_ok()
    );
}

use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use unlace::source::{Snapshot, SourceError};
struct State {
    bytes: Vec<u8>,
    revision: u128,
    available: usize,
    hole: Option<(usize, usize)>,
    fault: Option<usize>,
    fault_used: bool,
    chunk: usize,
    calls: u64,
    interrupt: bool,
    cancel_at: Option<u64>,
    cancel: Rc<AtomicBool>,
}
#[derive(Clone)]
struct Ranges(Rc<RefCell<State>>);
impl Ranges {
    fn new(bytes: Vec<u8>) -> Self {
        let available = bytes.len();
        Self(Rc::new(RefCell::new(State {
            bytes,
            revision: 1,
            available,
            hole: None,
            fault: None,
            fault_used: false,
            chunk: usize::MAX,
            calls: 0,
            interrupt: false,
            cancel_at: None,
            cancel: Rc::new(AtomicBool::new(false)),
        })))
    }
}
impl ReadAt for Ranges {
    fn snapshot(&mut self) -> io::Result<Snapshot> {
        let s = self.0.borrow();
        Ok(Snapshot {
            length: s.bytes.len() as u64,
            revision: s.revision,
        })
    }
    fn read_at(&mut self, offset: u64, bytes: &mut [u8]) -> io::Result<usize> {
        let mut s = self.0.borrow_mut();
        s.calls += 1;
        if s.cancel_at == Some(s.calls) {
            s.cancel.store(true, Ordering::Relaxed);
        }
        if s.interrupt && s.calls % 2 == 0 {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let offset = offset as usize;
        if offset >= s.available {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let mut len = bytes.len().min(s.available - offset).min(s.chunk);
        if let Some(fault) = s.fault {
            if offset == fault {
                s.fault_used = true;
                return Err(io::Error::other("injected positional source error"));
            }
            if offset < fault {
                len = len.min(fault - offset);
            }
        }
        if let Some((start, end)) = s.hole {
            if offset >= start && offset < end {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            if offset < start {
                len = len.min(start - offset);
            }
        }
        bytes[..len].copy_from_slice(&s.bytes[offset..offset + len]);
        Ok(len)
    }
}
fn ranged(source: Ranges, limits: SeekLimits) -> SeekReader<Ranges> {
    SeekReader::new(
        source,
        AudioSelection::Number(2),
        DemuxLimits::default(),
        limits,
    )
    .unwrap()
}

#[test]
fn metadata_and_indexes_resume_after_every_byte_of_short_interrupted_input() {
    let f = fixture_options(true, Some(2), 1, 32);
    for split in 0..f.bytes.len() {
        let source = Ranges::new(f.bytes.clone());
        {
            let mut s = source.0.borrow_mut();
            s.available = split;
            s.chunk = 3;
            s.interrupt = true;
        }
        let cancel = AtomicBool::new(false);
        let mut reader = ranged(source.clone(), SeekLimits::default());
        loop {
            match reader.prepare(&cancel) {
                Ok(Preparation::Ready) => break,
                Ok(Preparation::Yield) => (),
                Err(SeekError::Source(SourceError::NeedRange(extent))) => {
                    assert!(extent.end > extent.start);
                    source.0.borrow_mut().available = f.bytes.len();
                }
                result => panic!("split {split}: {result:?}"),
            }
        }
        let result = reader
            .seek(
                SeekRequest {
                    timestamp_ns: 280_000,
                    preroll_ns: 0,
                },
                &cancel,
            )
            .unwrap();
        assert_eq!(result.cluster_offset, f.clusters[1]);
    }
}

#[test]
fn packet_range_holes_at_every_suffix_byte_preserve_original_output() {
    let f = fixture_options(false, Some(2), 1, 32);
    let expected = sequential(&f.bytes);
    for split in f.clusters[0] as usize..f.bytes.len() {
        let source = Ranges::new(f.bytes.clone());
        source.0.borrow_mut().chunk = 3;
        let cancel = AtomicBool::new(false);
        let mut reader = ranged(source.clone(), SeekLimits::default());
        prepare(&mut reader, &cancel).unwrap();
        reader
            .seek(
                SeekRequest {
                    timestamp_ns: 180_000,
                    preroll_ns: 0,
                },
                &cancel,
            )
            .unwrap();
        source.0.borrow_mut().hole = Some((split, split + 1));
        let mut actual = Vec::new();
        loop {
            match reader.poll(&cancel) {
                Ok(SeekEvent::Packet(p)) => actual.push(packet(p)),
                Ok(SeekEvent::Yield) => (),
                Ok(SeekEvent::End(_)) => break,
                Err(SeekError::Source(SourceError::NeedRange(_))) => {
                    source.0.borrow_mut().hole = None
                }
                Err(error) => panic!("split {split}: {error}"),
            }
        }
        assert_eq!(actual, expected, "split {split}");
    }
}

#[test]
fn missing_destination_range_and_cancelled_seek_leave_buffered_packets_intact() {
    for cancellation in [false, true] {
        let f = fixture_options(false, Some(2), 1, 32);
        let expected = sequential(&f.bytes);
        let source = Ranges::new(f.bytes.clone());
        let cancel = source.0.borrow().cancel.clone();
        let mut reader = ranged(source.clone(), SeekLimits::default());
        prepare(&mut reader, &cancel).unwrap();
        let first = reader
            .seek(
                SeekRequest {
                    timestamp_ns: 180_000,
                    preroll_ns: 0,
                },
                &cancel,
            )
            .unwrap();
        loop {
            if let SeekEvent::Packet(p) = reader.poll(&cancel).unwrap() {
                assert_eq!(packet(p), expected[0]);
                break;
            }
        }
        if cancellation {
            let mut s = source.0.borrow_mut();
            s.cancel_at = Some(s.calls + 3);
        } else {
            source.0.borrow_mut().hole = Some((f.clusters[1] as usize, f.clusters[2] as usize));
        }
        let result = reader.seek(
            SeekRequest {
                timestamp_ns: 280_000,
                preroll_ns: 0,
            },
            &cancel,
        );
        if cancellation {
            assert!(matches!(result, Err(SeekError::Cancelled)));
        } else {
            assert!(matches!(
                result,
                Err(SeekError::Source(SourceError::NeedRange(_)))
            ));
        }
        source.0.borrow_mut().hole = None;
        source.0.borrow_mut().cancel_at = None;
        cancel.store(false, Ordering::Relaxed);
        assert_eq!(
            suffix(&mut reader, &cancel, first.cluster_offset),
            expected[1..]
        );
    }
}

#[test]
fn buffered_packet_publication_checks_snapshot_and_cancellation() {
    let f = fixture_options(false, Some(2), 1, 32);
    let expected = sequential(&f.bytes);
    let source = Ranges::new(f.bytes);
    let cancel = source.0.borrow().cancel.clone();
    let mut reader = ranged(source.clone(), SeekLimits::default());
    prepare(&mut reader, &cancel).unwrap();
    let result = reader
        .seek(
            SeekRequest {
                timestamp_ns: 180_000,
                preroll_ns: 0,
            },
            &cancel,
        )
        .unwrap();
    {
        let mut s = source.0.borrow_mut();
        s.cancel_at = Some(s.calls + 1);
    }
    assert!(matches!(reader.poll(&cancel), Err(SeekError::Cancelled)));
    source.0.borrow_mut().cancel_at = None;
    cancel.store(false, Ordering::Relaxed);
    assert_eq!(
        suffix(&mut reader, &cancel, result.cluster_offset),
        expected
    );
    reader
        .seek(
            SeekRequest {
                timestamp_ns: 180_000,
                preroll_ns: 0,
            },
            &cancel,
        )
        .unwrap();
    source.0.borrow_mut().revision += 1;
    assert!(matches!(
        reader.poll(&cancel),
        Err(SeekError::Source(SourceError::SourceChanged))
    ));
    source.0.borrow_mut().revision -= 1;
    assert!(matches!(
        reader.seek(
            SeekRequest {
                timestamp_ns: 180_000,
                preroll_ns: 0
            },
            &cancel
        ),
        Err(SeekError::Source(SourceError::SourceChanged))
    ));
}

#[test]
fn finite_truncations_and_forged_seekhead_offsets_never_prepare_successfully() {
    let f = fixture_options(false, Some(2), 1, 32);
    let cancel = AtomicBool::new(false);
    for len in 0..f.bytes.len() {
        let mut reader = reader(f.bytes[..len].to_vec());
        assert!(prepare(&mut reader, &cancel).is_err(), "truncation {len}");
    }
    for target in [0, u64::MAX] {
        let mut bytes = f.bytes.clone();
        let marker = [0x53, 0xac, 0x88];
        let pos = bytes.windows(3).position(|b| b == marker).unwrap();
        bytes[pos + 3..pos + 11].copy_from_slice(&target.to_be_bytes());
        let mut reader = reader(bytes);
        assert!(matches!(
            prepare(&mut reader, &cancel),
            Err(SeekError::Invalid { .. })
        ));
    }
}

#[test]
fn explicit_index_and_scan_limits_refuse_work_without_unbounded_reads() {
    let cancel = AtomicBool::new(false);
    for limits in [
        SeekLimits {
            max_entries: 2,
            ..SeekLimits::default()
        },
        SeekLimits {
            max_index_bytes: 1,
            ..SeekLimits::default()
        },
        SeekLimits {
            max_index_elements: 1,
            ..SeekLimits::default()
        },
    ] {
        let source = Ranges::new(fixture_options(false, Some(2), 1, 32).bytes);
        let mut reader = ranged(source, limits);
        assert!(matches!(
            prepare(&mut reader, &cancel),
            Err(SeekError::Limit(_))
        ));
    }
    let source = Ranges::new(fixture_options(false, Some(2), 1, 32).bytes);
    let mut reader = ranged(
        source,
        SeekLimits {
            max_scan_headers: 1,
            ..SeekLimits::default()
        },
    );
    prepare(&mut reader, &cancel).unwrap();
    assert!(matches!(
        reader.seek(
            SeekRequest {
                timestamp_ns: 180_000,
                preroll_ns: 0
            },
            &cancel
        ),
        Err(SeekError::Limit("scan headers"))
    ));
}

#[test]
fn cues_without_relative_hints_and_group_destinations_validate_actual_blocks() {
    let cancel = AtomicBool::new(false);
    // Replace the first relative-position field with an equal-sized Void;
    // the CueTime/track search must work independently of optional hints.
    let mut f = fixture_options(false, Some(2), 1, 32);
    let needle = number(0xf0, f.relative);
    let at = f
        .bytes
        .windows(needle.len())
        .position(|b| b == needle)
        .unwrap();
    f.bytes[at] = 0xec;
    let mut no_hint = reader(f.bytes);
    prepare(&mut no_hint, &cancel).unwrap();
    assert_eq!(
        no_hint
            .seek(
                SeekRequest {
                    timestamp_ns: 180_000,
                    preroll_ns: 0
                },
                &cancel
            )
            .unwrap()
            .cluster_offset,
        f.clusters[0]
    );
    let mut f = fixture_options(false, Some(2), 1, 32);
    // First selected SimpleBlock is 8 bytes, the other audio block is 7.
    let group_relative = f.relative
        + element(0xa3, &block(2, 0, 0x80, &[1, 10])).len() as u64
        + element(0xa3, &block(3, 0, 0x80, &[99])).len() as u64;
    change(&mut f.bytes, 0xf0, f.relative, group_relative);
    change(&mut f.bytes, 0xb3, 100, 120);
    let expected = sequential(&f.bytes);
    let mut group_hint = reader(f.bytes);
    prepare(&mut group_hint, &cancel).unwrap();
    let result = group_hint
        .seek(
            SeekRequest {
                timestamp_ns: 180_000,
                preroll_ns: 0,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(result.cue_time_ticks, Some(120));
    assert_eq!(result.actual_timestamp_ns, 99_993);
    assert_eq!(
        suffix(&mut group_hint, &cancel, result.cluster_offset),
        expected
    );
}

#[test]
fn cancellation_during_preparation_keeps_reusable_owned_state() {
    for call in [1, 5, 25, 80] {
        let f = fixture_options(true, Some(2), 1, 32);
        let source = Ranges::new(f.bytes);
        let cancel = source.0.borrow().cancel.clone();
        source.0.borrow_mut().cancel_at = Some(call);
        let mut reader = ranged(source.clone(), SeekLimits::default());
        assert!(matches!(
            prepare(&mut reader, &cancel),
            Err(SeekError::Cancelled)
        ));
        source.0.borrow_mut().cancel_at = None;
        cancel.store(false, Ordering::Relaxed);
        prepare(&mut reader, &cancel).unwrap();
        assert!(
            reader
                .seek(
                    SeekRequest {
                        timestamp_ns: 180_000,
                        preroll_ns: 0
                    },
                    &cancel
                )
                .is_ok()
        );
    }
}

#[test]
fn tiny_metadata_call_budgets_resume_and_prepare_steps_are_explicit() {
    use unlace::source::ReadLimits;
    let f = fixture_options(true, None, 1, 32);
    let source = Ranges::new(f.bytes);
    let cancel = AtomicBool::new(false);
    let mut reader = ranged(
        source,
        SeekLimits {
            read: ReadLimits {
                max_calls: 8,
                ..ReadLimits::default()
            },
            max_prepare_steps: 1,
            ..SeekLimits::default()
        },
    );
    let mut refusals = 0;
    loop {
        match reader.prepare(&cancel) {
            Ok(Preparation::Ready) => break,
            Ok(Preparation::Yield) => (),
            Err(SeekError::Source(SourceError::Limit)) => refusals += 1,
            error => panic!("{error:?}"),
        }
        assert!(reader.read_report().calls <= 8);
    }
    assert!(refusals > 0);
}

#[test]
fn well_formed_index_crc_is_unsupported_rather_than_corrupt() {
    let f = fixture_options(false, Some(2), 1, 32);
    let at = f
        .bytes
        .windows(4)
        .position(|b| b == [0x1c, 0x53, 0xbb, 0x6b])
        .unwrap();
    let width = f.bytes[at + 4].leading_zeros() as usize + 1;
    let mut len = usize::from(f.bytes[at + 4]) & (0xffusize >> width);
    for byte in &f.bytes[at + 5..at + 4 + width] {
        len = len * 256 + usize::from(*byte);
    }
    let payload = at + 4 + width;
    let mut contents = element(0xbf, &[0, 0, 0, 0]);
    contents.extend_from_slice(&f.bytes[payload..payload + len]);
    let data_offset = (f.clusters[0] - f.cue_positions[0]) as usize;
    let mut segment = f.bytes[data_offset..at].to_vec();
    segment.extend(element(0x1c53bb6b, &contents));
    segment.extend_from_slice(&f.bytes[payload + len..]);
    let segment_at = f
        .bytes
        .windows(4)
        .position(|b| b == [0x18, 0x53, 0x80, 0x67])
        .unwrap();
    let mut bytes = f.bytes[..segment_at].to_vec();
    bytes.extend(element(0x18538067, &segment));
    let mut reader = reader(bytes);
    assert!(matches!(
        prepare(&mut reader, &AtomicBool::new(false)),
        Err(SeekError::Unsupported("CRC-32 verification"))
    ));
}

#[test]
fn fallback_does_not_reclassify_missing_cluster_timestamp_as_no_access_point() {
    let f = fixture_options(false, None, 1, 32);
    let data_offset = (f.clusters[0] - f.cue_positions[0]) as usize;
    let segment_at = f
        .bytes
        .windows(4)
        .position(|b| b == [0x18, 0x53, 0x80, 0x67])
        .unwrap();
    let mut segment = f.bytes[data_offset..f.clusters[0] as usize].to_vec();
    segment.extend(element(0x1f43b675, &[]));
    let mut bytes = f.bytes[..segment_at].to_vec();
    bytes.extend(element(0x18538067, &segment));
    let cancel = AtomicBool::new(false);
    let mut reader = reader(bytes);
    prepare(&mut reader, &cancel).unwrap();
    assert!(matches!(
        reader.seek(
            SeekRequest {
                timestamp_ns: 0,
                preroll_ns: 0
            },
            &cancel
        ),
        Err(SeekError::Invalid {
            reason: "missing Cluster Timestamp",
            ..
        })
    ));
}

#[test]
fn source_errors_at_every_prepare_byte_have_deterministic_failure_or_retry_scope() {
    let f = fixture_options(false, Some(2), 1, 32);
    let cancel = AtomicBool::new(false);
    for fault in 0..f.bytes.len() {
        let source = Ranges::new(f.bytes.clone());
        source.0.borrow_mut().fault = Some(fault);
        let mut reader = ranged(source.clone(), SeekLimits::default());
        match prepare(&mut reader, &cancel) {
            Ok(()) => assert!(!source.0.borrow().fault_used),
            Err(SeekError::Source(SourceError::Io {
                offset: Some(offset),
                error,
            })) => {
                assert_eq!(offset, fault as u64);
                assert_eq!(error.kind(), io::ErrorKind::Other);
                assert!(source.0.borrow().fault_used);
                assert!(reader.read_report().calls <= SeekLimits::default().read.max_calls);
                source.0.borrow_mut().fault = None;
                if (fault as u64) < f.clusters[0] {
                    let calls = source.0.borrow().calls;
                    assert!(matches!(reader.prepare(&cancel), Err(SeekError::Failed)));
                    assert_eq!(source.0.borrow().calls, calls);
                } else {
                    // Index reads are positional transactions; retrying never
                    // imports new metadata or changes an active packet cursor.
                    prepare(&mut reader, &cancel).unwrap();
                    assert_eq!(
                        reader
                            .seek(
                                SeekRequest {
                                    timestamp_ns: 180_000,
                                    preroll_ns: 0
                                },
                                &cancel
                            )
                            .unwrap()
                            .cluster_offset,
                        f.clusters[0]
                    );
                }
            }
            Err(error) => panic!("fault {fault}: {error}"),
        }
    }
}

#[test]
fn source_errors_at_every_suffix_byte_preserve_location_and_terminal_packet_state() {
    let f = fixture_options(false, Some(2), 1, 32);
    let cancel = AtomicBool::new(false);
    for fault in f.clusters[0] as usize..f.bytes.len() {
        let source = Ranges::new(f.bytes.clone());
        let mut reader = ranged(source.clone(), SeekLimits::default());
        prepare(&mut reader, &cancel).unwrap();
        reader
            .seek(
                SeekRequest {
                    timestamp_ns: 180_000,
                    preroll_ns: 0,
                },
                &cancel,
            )
            .unwrap();
        source.0.borrow_mut().fault = Some(fault);
        loop {
            match reader.poll(&cancel) {
                Ok(SeekEvent::Packet(_) | SeekEvent::Yield) => (),
                Ok(SeekEvent::End(_)) => panic!("source error at {fault} became successful End"),
                Err(SeekError::Source(SourceError::Io {
                    offset: Some(offset),
                    error,
                })) => {
                    assert_eq!(offset, fault as u64);
                    assert_eq!(error.kind(), io::ErrorKind::Other);
                    let calls = source.0.borrow().calls;
                    assert!(matches!(reader.poll(&cancel), Err(SeekError::Failed)));
                    assert_eq!(source.0.borrow().calls, calls);
                    break;
                }
                Err(error) => panic!("fault {fault}: {error}"),
            }
        }
    }
}

#[test]
fn wide_before_first_cue_scan_reports_read_limit_and_preserves_active_suffix() {
    let f = fixture_options(false, None, 1, 32);
    let data_offset = (f.clusters[0] - f.cue_positions[0]) as usize;
    let segment_at = f
        .bytes
        .windows(4)
        .position(|b| b == [0x18, 0x53, 0x80, 0x67])
        .unwrap();
    let mut segment = f.bytes[data_offset..f.clusters[0] as usize].to_vec();
    let first_relative = segment.len() as u64;
    // Five Clusters are the smallest count for this fixture and 50-call
    // allowance: four complete the fallback scan without a refusal.
    for i in 0..5 {
        let mut cluster = f.bytes[f.clusters[0] as usize..f.clusters[1] as usize].to_vec();
        change(&mut cluster, 0xe7, 100, i * 100);
        segment.extend(cluster);
    }
    let mut position = number(0xf7, 2);
    position.extend(number(0xf1, first_relative));
    position.extend(number(0xf0, f.relative));
    let mut point = number(0xb3, 0);
    point.extend(element(0xb7, &position));
    segment.extend(element(0x1c53bb6b, &element(0xbb, &point)));
    let mut bytes = f.bytes[..segment_at].to_vec();
    bytes.extend(element(0x18538067, &segment));
    let expected = sequential(&bytes);
    let cancel = AtomicBool::new(false);
    let source = Ranges::new(bytes);
    let mut reader = ranged(
        source,
        SeekLimits {
            read: unlace::source::ReadLimits {
                max_calls: 50,
                ..Default::default()
            },
            ..SeekLimits::default()
        },
    );
    loop {
        match reader.prepare(&cancel) {
            Ok(Preparation::Ready) => break,
            Ok(Preparation::Yield) => (),
            Err(SeekError::Source(SourceError::Limit)) => {
                assert_eq!(reader.read_report().calls, 50)
            }
            Err(error) => panic!("{error}"),
        }
    }
    let first = reader
        .seek(
            SeekRequest {
                timestamp_ns: 80_000,
                preroll_ns: 0,
            },
            &cancel,
        )
        .unwrap();
    let mut actual = Vec::new();
    while actual.len() < 2 {
        match reader.poll(&cancel) {
            Ok(SeekEvent::Packet(p)) => actual.push(packet(p)),
            Ok(SeekEvent::Yield) | Err(SeekError::Source(SourceError::Limit)) => (),
            _ => panic!("could not retain a pending laced packet"),
        }
    }
    assert_eq!(actual[1].lace, (0, 2));
    assert!(matches!(
        reader.seek(
            SeekRequest {
                timestamp_ns: 0,
                preroll_ns: 0
            },
            &cancel
        ),
        Err(SeekError::Source(SourceError::Limit))
    ));
    assert_eq!(reader.read_report().calls, 50);
    match reader.poll(&cancel).unwrap() {
        SeekEvent::Packet(p) => {
            let p = packet(p);
            assert_eq!(p.lace, (1, 2));
            actual.push(p);
        }
        _ => panic!("pending laced packet was lost"),
    }
    assert_eq!(reader.read_report().calls, 0);
    loop {
        match reader.poll(&cancel) {
            Ok(SeekEvent::Packet(p)) => actual.push(packet(p)),
            Ok(SeekEvent::Yield) => (),
            Ok(SeekEvent::End(end)) => {
                assert_eq!(end.from_cluster_offset, first.cluster_offset);
                break;
            }
            Err(SeekError::Source(SourceError::Limit)) => {
                assert_eq!(reader.read_report().calls, 50)
            }
            Err(error) => panic!("{error}"),
        }
    }
    assert_eq!(actual, expected);
}
