//! Optional writer state fuzz executable. Input format writer-v1 is documented
//! in README.md. Assertions and unwraps are test-only state/output oracles.
#![cfg(test)]
#![forbid(unsafe_code)]
use std::{
    cell::Cell,
    io::{self, Read, Seek, SeekFrom, Write},
    rc::Rc,
};
use unlace::{
    block::{BlockLimits, Lacing},
    demux::*,
    write::*,
};
const MAX_OUTPUT: usize = 128 * 1024;
#[derive(Default)]
struct Calls {
    total: Cell<u64>,
    write: Cell<u64>,
    seek: Cell<u64>,
    flush: Cell<u64>,
}
struct Sink {
    data: Vec<u8>,
    position: usize,
    calls: Rc<Calls>,
    mode: u8,
    at: u64,
    chunk: usize,
}
impl Sink {
    fn hit(&self, operation: u8) -> bool {
        self.calls.total.set(self.calls.total.get() + 1);
        let counter = match operation {
            0 => &self.calls.write,
            1 => &self.calls.seek,
            _ => &self.calls.flush,
        };
        counter.set(counter.get() + 1);
        if self.mode == 10 {
            return true;
        }
        let expected = match self.mode {
            3 | 11 => 2,
            4 | 8 | 12 => 1,
            _ => 0,
        };
        operation == expected && counter.get() == self.at
    }
    fn error() -> io::Error {
        io::Error::other("injected writer sink error")
    }
}
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.hit(0) {
            match self.mode {
                1 => return Err(Self::error()),
                2 => return Ok(0),
                5 | 10 | 13 => return Err(io::ErrorKind::Interrupted.into()),
                6 => return Err(io::ErrorKind::WouldBlock.into()),
                7 => return Ok(bytes.len() + 1),
                _ => (),
            }
        }
        let count = bytes.len().min(self.chunk);
        let end = self
            .position
            .checked_add(count)
            .filter(|&n| n <= MAX_OUTPUT)
            .ok_or_else(Self::error)?;
        if self.data.len() < end {
            self.data.resize(end, 0);
        }
        self.data[self.position..end].copy_from_slice(&bytes[..count]);
        self.position = end;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.hit(2) {
            match self.mode {
                3 => return Err(Self::error()),
                10 | 11 => return Err(io::ErrorKind::Interrupted.into()),
                _ => (),
            }
        }
        Ok(())
    }
}
impl Seek for Sink {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let hit = self.hit(1);
        if hit {
            match self.mode {
                4 => return Err(Self::error()),
                10 | 12 => return Err(io::ErrorKind::Interrupted.into()),
                _ => (),
            }
        }
        let value = match from {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(n) => self.position as i128 + i128::from(n),
            SeekFrom::End(n) => self.data.len() as i128 + i128::from(n),
        };
        let position = usize::try_from(value)
            .ok()
            .filter(|&n| n <= MAX_OUTPUT)
            .ok_or_else(Self::error)?;
        self.position = position;
        Ok(position as u64 + u64::from(hit && self.mode == 8))
    }
}
enum Writer {
    Live(LiveWriter<Sink>),
    Indexed(IndexedWriter<Sink>),
}
impl Writer {
    fn block(&mut self, b: EncodedBlock<'_>) -> Result<(), WriteError> {
        match self {
            Self::Live(w) => w.write_block(b),
            Self::Indexed(w) => w.write_block(b),
        }
    }
    fn flush(&mut self) -> Result<(), WriteError> {
        match self {
            Self::Live(w) => w.flush(),
            Self::Indexed(w) => w.flush(),
        }
    }
    fn finish(&mut self) -> Result<WriteReceipt, WriteError> {
        match self {
            Self::Live(w) => w.finish(),
            Self::Indexed(w) => w.finish(),
        }
    }
    fn state(&self) -> WriteState {
        match self {
            Self::Live(w) => w.state(),
            Self::Indexed(w) => w.state(),
        }
    }
    fn position(&self) -> u64 {
        match self {
            Self::Live(w) => w.position(),
            Self::Indexed(w) => w.position(),
        }
    }
    fn buffered(&self) -> usize {
        match self {
            Self::Live(w) => w.buffered_bytes(),
            Self::Indexed(w) => w.buffered_bytes(),
        }
    }
    fn sink(self) -> Sink {
        match self {
            Self::Live(w) => w.into_inner(),
            Self::Indexed(w) => w.into_inner(),
        }
    }
}
struct Input<'a> {
    data: &'a [u8],
    position: usize,
}
impl<'a> Input<'a> {
    fn byte(&mut self) -> u8 {
        let b = self.data.get(self.position).copied().unwrap_or(0);
        self.position = self.position.saturating_add(1);
        b
    }
    fn take(&mut self, len: usize) -> &'a [u8] {
        let start = self.position.min(self.data.len());
        let end = start.saturating_add(len).min(self.data.len());
        self.position = end;
        &self.data[start..end]
    }
    fn ticks(&mut self) -> i64 {
        let mut bytes = [0; 8];
        for b in &mut bytes {
            *b = self.byte();
        }
        i64::from_le_bytes(bytes)
    }
}
#[derive(Debug, PartialEq)]
struct Expected {
    data: Vec<u8>,
    track: u64,
    timestamp: Option<i128>,
    raw: i128,
    padding: i64,
    once: i64,
    duration: Option<i128>,
    default_duration: Option<u64>,
    lace: (u16, u16),
    key: bool,
    invisible: bool,
    discardable: bool,
}
fn observe(
    out: &[u8],
    track: u64,
    expected: &[Expected],
    completion: Completion,
    tracks: &[TrackConfig<'_>],
) {
    let mut reader =
        Demuxer::new(out, AudioSelection::Number(track), DemuxLimits::default()).unwrap();
    let mut index = 0;
    let mut ended = false;
    for _ in 0..20_000 {
        match reader.poll().unwrap() {
            DemuxEvent::TracksReady => {
                assert_eq!(reader.tracks().len(), tracks.len());
                for (a, b) in reader.tracks().iter().zip(tracks) {
                    assert_eq!(a.number, b.number);
                    assert_eq!(a.uid, b.uid);
                    assert_eq!(a.codec_id, b.codec_id);
                    assert_eq!(a.codec_private, b.codec_private);
                    assert_eq!(a.codec_delay_ns, b.codec_delay_ns);
                    assert_eq!(a.seek_preroll_ns, b.seek_preroll_ns);
                    assert_eq!(a.enabled, b.enabled);
                    assert_eq!(a.lacing_allowed, b.lacing_allowed);
                    assert_eq!(a.default_duration_ns, b.default_duration_ns);
                    match b.media {
                        MediaParameters::Audio(b) => {
                            let a = a.audio.unwrap();
                            assert_eq!(a.sampling_frequency, b.sampling_frequency);
                            assert_eq!(a.output_sampling_frequency, b.output_sampling_frequency);
                            assert_eq!(a.channels, b.channels);
                            assert_eq!(a.bit_depth, b.bit_depth);
                        }
                        MediaParameters::Video(_) => assert!(a.audio.is_none()),
                    }
                }
            }
            DemuxEvent::Packet(p) => {
                let e = expected
                    .iter()
                    .filter(|p| p.track == track)
                    .nth(index)
                    .expect("extra output packet");
                index += 1;
                assert_eq!(
                    &Expected {
                        data: p.data.to_vec(),
                        track: p.track_number,
                        timestamp: p.timestamp_ns,
                        raw: p.block_timestamp_ns,
                        padding: p.block_discard_padding_ns,
                        once: p.discard_padding_ns,
                        duration: p.block_duration_ns,
                        default_duration: p.default_duration_ns,
                        lace: (p.lace_index, p.lace_count),
                        key: p.keyframe,
                        invisible: p.invisible,
                        discardable: p.discardable
                    },
                    e
                );
            }
            DemuxEvent::End(c) => {
                assert_eq!(c, completion);
                ended = true;
                break;
            }
            DemuxEvent::Yield => (),
            DemuxEvent::NeedInput => panic!("in-memory output unavailable"),
        }
    }
    assert!(ended, "output did not terminate");
    assert_eq!(index, expected.iter().filter(|p| p.track == track).count());
}
fn invariant(
    w: &mut Writer,
    before: (u64, usize, u64),
    calls: &Calls,
    result: &Result<(), WriteError>,
    maximum: u64,
) {
    assert!(
        calls.total.get() - before.2 <= maximum,
        "I/O allowance exceeded"
    );
    match result {
        Err(
            WriteError::Io { .. } | WriteError::Position { .. } | WriteError::Limit("I/O calls"),
        ) => {
            assert_eq!(
                w.state(),
                WriteState::Failed,
                "I/O error did not fail writer"
            );
        }
        Err(WriteError::Invalid(_) | WriteError::Unsupported(_) | WriteError::OpusHead(_)) => {
            assert_eq!(w.state(), WriteState::Writing, "preflight poisoned writer");
        }
        Err(WriteError::State(state)) => assert_eq!(w.state(), *state),
        Err(_) => assert_ne!(w.state(), WriteState::Finished, "error claimed completion"),
        Ok(()) => (),
    }
    if result.is_err() && w.state() == WriteState::Writing {
        assert_eq!(
            (w.position(), w.buffered(), calls.total.get()),
            before,
            "preflight changed observable output"
        );
    }
    if w.state() != WriteState::Writing {
        let state = w.state();
        let n = calls.total.get();
        assert!(
            matches!(w.block(EncodedBlock::new(1,0,b"x")),Err(WriteError::State(s)) if s==state)
        );
        assert!(matches!(w.flush(),Err(WriteError::State(s)) if s==state));
        assert!(matches!(w.finish(),Err(WriteError::State(s)) if s==state));
        assert_eq!(calls.total.get(), n);
    }
}
fn run(
    data: &[u8],
    indexed: bool,
    faults: bool,
    emit: Option<&std::path::Path>,
) -> Option<(u64, u64)> {
    let mut input = Input { data, position: 0 };
    let mut control = [0u8; 20];
    for b in &mut control {
        *b = input.byte();
    }
    let scale = [1, 1000, 1_000_000, u64::MAX][usize::from(control[0] & 3)];
    let mut private = *b"OpusHead\x01\x01\x38\x01\x80\xbb\0\0\0\0\0";
    if control[12] & 1 != 0 {
        private[8] = control[13];
    }
    if control[12] & 2 != 0 {
        private[18] = control[14];
    }
    let default = (control[0] & 4 == 0).then_some(20_000_000);
    let mut tracks = vec![TrackConfig {
        number: 1,
        uid: 1,
        codec_id: "A_OPUS",
        codec_private: &private,
        media: MediaParameters::Audio(AudioParameters {
            sampling_frequency: 48000.0,
            output_sampling_frequency: 48000.0,
            channels: 1,
            bit_depth: None,
        }),
        codec_delay_ns: 6500000,
        seek_preroll_ns: 80000000,
        default_duration_ns: default,
        lacing_allowed: control[0] & 8 == 0,
        enabled: true,
    }];
    if control[0] & 16 != 0 {
        let mut t = tracks[0];
        t.number = 2;
        t.uid = 2;
        t.enabled = control[0] & 32 == 0;
        tracks.push(t);
    }
    if let Some(t) = tracks.get_mut(1) {
        match control[17] % 7 {
            0 => (),
            1 | 2 => {
                t.codec_id = "A_VORBIS";
                t.codec_private = if control[17] % 7 == 1 {
                    b"\x02\x07\x07\x01vorbis\x03vorbis\x05vorbis"
                } else {
                    b"\x02\xff\x07"
                };
                t.codec_delay_ns = 0;
            }
            3 | 4 | 6 => {
                t.codec_id = match control[17] % 7 {
                    3 => "V_VP8",
                    4 => "V_VP9",
                    _ => "V_UNKNOWN",
                };
                t.codec_private = if control[18] & 1 == 0 { &[] } else { b"opaque" };
                t.codec_delay_ns = 0;
                t.media = MediaParameters::Video(VideoParameters {
                    pixel_width: if control[18] & 2 == 0 { 64 } else { 0 },
                    pixel_height: 64,
                });
            }
            _ => t.codec_id = "A_UNKNOWN",
        }
    }
    if control[12] & 4 != 0 {
        tracks[0].codec_delay_ns = u64::from(control[15]);
    }
    let maximum = [1, 16, 128, 1024][usize::from(control[9] & 3)];
    let limits = WriteLimits {
        block: BlockLimits {
            max_block_size: 4096,
            max_frame_size: [0, 16, 256, 1024][usize::from((control[8] >> 2) & 3)],
            max_frames: 8,
            max_lace_header_size: [0, 8, 128, 1024][usize::from(control[8] & 3)],
        },
        max_cluster_buffer: [0, 17, 128, 1024][usize::from(control[5] & 3)],
        max_tracks: if control[9] & 4 != 0 { 1 } else { 2 },
        max_metadata_size: [0, 64, 512, 4096][usize::from(control[6] & 3)],
        max_cues: [0, 1, 4, 32][usize::from(control[7] & 3)],
        max_index_size: 2048,
        max_io_calls: maximum,
    };
    let config = WriteConfig {
        timestamp_scale_ns: scale,
        writing_app: match control[11] % 3 {
            0 => "writer fuzz",
            1 => "",
            _ => "\0",
        },
        cue_interval_ns: if control[10] & 1 == 0 {
            0
        } else {
            1_000_000_000
        },
        max_cluster_duration_ns: [0, 20_000_000, 1_000_000_000, u64::MAX]
            [usize::from(control[4] & 3)],
        limits,
    };
    let calls = Rc::new(Calls::default());
    let sink = Sink {
        data: Vec::new(),
        position: 0,
        calls: Rc::clone(&calls),
        mode: if faults { control[1] % 14 } else { 0 },
        at: 1 + u64::from(control[2] % 64),
        chunk: if control[3] == 0 {
            usize::MAX
        } else {
            usize::from(control[3] % 32) + 1
        },
    };
    let created = if indexed {
        IndexedWriter::new(sink, &tracks, config).map(Writer::Indexed)
    } else {
        LiveWriter::new(sink, &tracks, config).map(Writer::Live)
    };
    assert!(calls.total.get() <= maximum);
    let Ok(mut w) = created else { return None };
    let mut expected = Vec::new();
    let mut blocks = 0u64;
    let mut last_raw = None;
    let mut last_frame = [None, None];
    let mut receipt = None;
    let mut aborted = false;
    for _ in 0..32 {
        if input.position >= data.len() {
            break;
        }
        let tag = input.byte();
        let before = (w.position(), w.buffered(), calls.total.get());
        let result = match tag & 3 {
            1 => w.flush(),
            2 => {
                let value = w.finish();
                if let Ok(r) = &value {
                    assert_eq!(w.state(), WriteState::Finished, "finish did not terminate");
                    receipt = Some(*r);
                }
                value.map(|_| ())
            }
            3 => {
                aborted = true;
                break;
            }
            _ => {
                let number = 1 + u64::from(input.byte() % 3);
                let ticks = input.ticks();
                let flags = input.byte();
                let count = usize::from(input.byte() % 11);
                let lace = match input.byte() & 3 {
                    0 => Lacing::None,
                    1 => Lacing::Fixed,
                    2 => Lacing::Xiph,
                    _ => Lacing::Ebml,
                };
                let pad = i64::from(input.byte() as i8) * 1_000_000;
                let d = input.byte();
                let references = input.byte();
                let payload_len = usize::from(input.byte()) * 4;
                let payload = input.take(payload_len);
                let frames: Vec<&[u8]> = (0..count)
                    .map(|i| {
                        if lace == Lacing::Fixed && flags & 64 == 0 {
                            payload
                        } else {
                            &payload[..(i * 13 + usize::from(flags)) % (payload.len() + 1)]
                        }
                    })
                    .collect();
                let frame_slices = if flags & 8 == 0 {
                    Frames::Single(payload)
                } else {
                    Frames::Laced {
                        frames: &frames,
                        lacing: lace,
                    }
                };
                let timestamp = if tag & 128 != 0 {
                    if flags & 16 == 0 {
                        i128::MAX
                    } else {
                        i128::MIN
                    }
                } else {
                    let ticks = if control[16] & 1 != 0 {
                        i128::from(ticks as u64)
                    } else {
                        i128::from(ticks)
                    };
                    let delay = tracks
                        .get((number - 1) as usize)
                        .map_or(6_500_000, |t| t.codec_delay_ns);
                    ticks
                        .checked_mul(i128::from(scale))
                        .and_then(|n| n.checked_sub(i128::from(delay)))
                        .unwrap_or(i128::MAX)
                };
                let refs = [i64::from(references & 63) - 32, -20, 20];
                let ref_count = usize::from((references >> 6) & 3);
                let b = EncodedBlock {
                    track_number: number,
                    timestamp_ns: timestamp,
                    frames: frame_slices,
                    duration_ns: if d == 0 {
                        None
                    } else if flags & 32 != 0 {
                        Some(0)
                    } else {
                        Some(u64::from(d).checked_mul(scale).unwrap_or(u64::MAX))
                    },
                    discard_padding_ns: pad,
                    keyframe: flags & 1 == 0,
                    invisible: flags & 2 != 0,
                    discardable: flags & 4 != 0,
                    references: &refs[..ref_count],
                };
                let result = w.block(b);
                if result.is_ok() {
                    let ti = (number - 1) as usize;
                    let track = tracks.get(ti).expect("accepted unknown track");
                    let raw = timestamp
                        .checked_add(i128::from(track.codec_delay_ns))
                        .unwrap();
                    assert_eq!(raw % i128::from(scale), 0);
                    assert!(last_raw.is_none_or(|n| raw >= n));
                    assert!(last_frame[ti].is_none_or(|n| timestamp >= n));
                    assert!(!b.keyframe || b.references.is_empty());
                    assert!(!b.discardable || (d == 0 && pad == 0 && ref_count == 0));
                    let single = [payload];
                    let selected: &[&[u8]] = if flags & 8 == 0 { &single } else { &frames };
                    assert!(!selected.is_empty() && selected.len() <= 8);
                    assert!(
                        flags & 8 == 0
                            || (lace != Lacing::None
                                && selected.len() >= 2
                                && track.lacing_allowed)
                    );
                    if lace == Lacing::Fixed && flags & 8 != 0 {
                        assert!(selected.iter().all(|p| p.len() == selected[0].len()));
                    }
                    let known = b
                        .duration_ns
                        .map(i128::from)
                        .or_else(|| default.map(|n| i128::from(n) * selected.len() as i128));
                    if let Some(n) = known {
                        assert!(i128::from(pad).abs() <= n);
                    }
                    for (i, p) in selected.iter().enumerate() {
                        expected.push(Expected {
                            data: p.to_vec(),
                            track: number,
                            timestamp: if i == 0 {
                                Some(timestamp)
                            } else {
                                default.map(|n| timestamp + i as i128 * i128::from(n))
                            },
                            raw,
                            padding: pad,
                            once: if (pad < 0 && i == 0) || (pad > 0 && i + 1 == selected.len()) {
                                pad
                            } else {
                                0
                            },
                            duration: b.duration_ns.map(i128::from),
                            default_duration: default,
                            lace: (i as u16, selected.len() as u16),
                            key: b.keyframe,
                            invisible: b.invisible,
                            discardable: b.discardable,
                        });
                    }
                    last_frame[ti] = if selected.len() == 1 {
                        Some(timestamp)
                    } else {
                        default.map(|n| timestamp + (selected.len() - 1) as i128 * i128::from(n))
                    };
                    last_raw = Some(raw);
                    blocks += 1;
                }
                result
            }
        };
        invariant(&mut w, before, &calls, &result, maximum);
        assert!(w.buffered() <= limits.max_cluster_buffer);
    }
    if w.state() == WriteState::Writing && !aborted {
        let before = (w.position(), w.buffered(), calls.total.get());
        let result = w.finish();
        if let Ok(r) = &result {
            assert_eq!(w.state(), WriteState::Finished, "finish did not terminate");
            receipt = Some(*r);
        }
        invariant(&mut w, before, &calls, &result.map(|_| ()), maximum);
    }
    let outcome = receipt.map(|r| (r.blocks, r.packets));
    let sink = w.sink();
    assert!(sink.data.len() <= MAX_OUTPUT);
    if let Some(r) = receipt {
        assert_eq!(r.packets, expected.len() as u64);
        assert_eq!(r.blocks, blocks);
        assert!(r.clusters > 0 && r.clusters <= blocks);
        assert_eq!(r.end_offset, sink.data.len() as u64);
        assert_eq!(
            r.completion,
            if indexed {
                OutputCompletion::Indexed
            } else {
                OutputCompletion::Live
            }
        );
        for track in &tracks {
            if track.enabled && matches!(track.media, MediaParameters::Audio(_)) {
                observe(
                    &sink.data,
                    track.number,
                    &expected,
                    if indexed {
                        Completion::FiniteSegment
                    } else {
                        Completion::UnknownSizeSegment
                    },
                    &tracks,
                );
            }
        }
        if let Some(root) = emit {
            let name = format!(
                "{}-{}.webm",
                if indexed { "indexed" } else { "live" },
                if faults { "fault" } else { "clean" }
            );
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(root.join(name))
                .unwrap();
            file.write_all(&sink.data).unwrap();
        }
    }
    outcome
}
// This lane must accept every generated block. Its expected counts do not
// depend on returned Ok results, unlike the intentionally invalid action lane.
fn valid_input(data: &[u8]) -> (Vec<u8>, u64) {
    let byte = |i: usize| data.get(i).copied().unwrap_or(0);
    let count = 2 + byte(0) % 7;
    let mut out = vec![0; 20];
    out[0] = 2;
    out[3] = byte(1) % 32;
    out[4] = byte(2) & 3;
    out[5] = byte(3) & 3;
    out[6] = 3;
    out[7] = 3;
    out[8] = 15;
    out[9] = 3;
    let payload: Vec<u8> = (0..64).map(|i| byte(i + 4)).collect();
    for i in 0..3i64 {
        let laced = i == 1;
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&(i * 200).to_le_bytes());
        out.extend_from_slice(&[
            if laced { 8 } else { 0 },
            if laced { count } else { 0 },
            if laced { 1 + byte(4) % 3 } else { 0 },
            if laced {
                (i16::from(byte(5) % 39) - 19) as i8 as u8
            } else {
                0
            },
            0,
            0,
            16,
        ]);
        out.extend_from_slice(&payload);
        if byte(i as usize + 6) & 1 != 0 {
            out.push(1);
        }
    }
    out.push(2);
    (out, 2 + u64::from(count))
}
fn main() {
    let mut args = std::env::args_os().skip(1);
    let emit = match args.next() {
        None => None,
        Some(flag) => {
            assert_eq!(flag, "--emit");
            let root = std::path::PathBuf::from(args.next().unwrap())
                .canonicalize()
                .unwrap();
            let allowed = std::env::current_dir()
                .unwrap()
                .join("runs")
                .canonicalize()
                .unwrap();
            assert!(root.starts_with(&allowed) && root != allowed);
            assert!(args.next().is_none());
            Some(root)
        }
    };
    let mut data = Vec::new();
    io::stdin().take(8194).read_to_end(&mut data).unwrap();
    assert!(data.len() <= 8193);
    for indexed in [false, true] {
        let _ = run(&data, indexed, false, emit.as_deref());
        let _ = run(&data, indexed, true, emit.as_deref());
    }
    let (valid, packets) = valid_input(&data);
    for indexed in [false, true] {
        assert_eq!(
            run(&valid, indexed, false, None),
            Some((3, packets)),
            "guaranteed-valid sequence refused or miscounted"
        );
    }
}
