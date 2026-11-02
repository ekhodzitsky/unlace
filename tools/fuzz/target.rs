//! Optional bounded fuzz test executable, built directly by tools/fuzz/run.py.
//! Assertions and unwraps are test oracles, never a production recovery path.
//! Five input modes cover scalar parsers, framing, demux, seeking and source state.
#![cfg(test)]
#![forbid(unsafe_code)]
use std::io::{self, Cursor, Read};
use std::sync::atomic::AtomicBool;
use unlace::{block::*, demux::*, seek::*, source::*, stream};

struct Chunk<'a> {
    bytes: &'a [u8],
    position: usize,
    chunk: usize,
    pause: usize,
    paused: bool,
    interrupt: bool,
}
impl Read for Chunk<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.interrupt {
            self.interrupt = false;
            return Err(io::ErrorKind::Interrupted.into());
        }
        self.interrupt = true;
        if !self.paused && self.position == self.pause {
            self.paused = true;
            return Err(io::ErrorKind::WouldBlock.into());
        }
        if self.position == self.bytes.len() {
            return Ok(0);
        }
        let mut n = out
            .len()
            .min(self.chunk)
            .min(self.bytes.len() - self.position);
        if !self.paused && self.position < self.pause {
            n = n.min(self.pause - self.position);
        }
        out[..n].copy_from_slice(&self.bytes[self.position..self.position + n]);
        self.position += n;
        Ok(n)
    }
}
fn chunk(bytes: &[u8]) -> Chunk<'_> {
    Chunk {
        bytes,
        position: 0,
        chunk: 1 + bytes.first().copied().unwrap_or(0) as usize % 7,
        pause: if bytes.is_empty() {
            0
        } else {
            bytes
                .iter()
                .fold(0usize, |a, b| a.wrapping_add(*b as usize))
                % (bytes.len() + 1)
        },
        paused: false,
        interrupt: true,
    }
}
fn framing() -> stream::Limits {
    stream::Limits {
        max_depth: 16,
        max_metadata_size: 8192,
        max_payload_size: 8192,
        max_skip_size: 8192,
        max_read_calls: 8,
        max_read_bytes: 64,
        ..Default::default()
    }
}
fn demux_limits() -> DemuxLimits {
    DemuxLimits {
        framing: framing(),
        block: BlockLimits {
            max_block_size: 8192,
            max_frame_size: 8192,
            max_frames: 32,
            max_lace_header_size: 512,
        },
        max_tracks: 8,
        max_metadata_size: 16384,
        max_events_per_poll: 8,
    }
}
fn packet(trace: &mut Vec<u8>, p: Packet<'_>) {
    trace.push(7);
    trace.extend((p.data.len() as u64).to_le_bytes());
    trace.extend(p.data);
    trace.extend(
        format!(
            "{:?}",
            (
                p.timestamp_ns,
                p.block_timestamp_ns,
                p.track_number,
                p.block_offset,
                p.lace_index,
                p.lace_count,
                p.block_duration_ns,
                p.block_discard_padding_ns,
                p.discard_padding_ns,
                p.default_duration_ns
            )
        )
        .bytes(),
    );
    trace.extend([
        u8::from(p.keyframe),
        u8::from(p.invisible),
        u8::from(p.discardable),
    ]);
}
fn demux<R: Read>(source: R) -> (Vec<u8>, String, u64) {
    let mut d = Demuxer::new(source, AudioSelection::First, demux_limits()).unwrap();
    let mut trace = Vec::new();
    for _ in 0..100_000 {
        match d.poll() {
            Ok(DemuxEvent::TracksReady) => {
                trace.push(1);
                trace.extend(format!("{:?}{:?}", d.tracks(), d.info()).bytes());
            }
            Ok(DemuxEvent::Packet(p)) => packet(&mut trace, p),
            Ok(DemuxEvent::Yield | DemuxEvent::NeedInput) => (),
            Ok(DemuxEvent::End(e)) => return (trace, format!("{e:?}"), d.position()),
            Err(e) => {
                let pos = d.position();
                assert!(matches!(d.poll(), Err(DemuxError::Failed)));
                assert_eq!(d.position(), pos);
                return (trace, format!("{e:?}"), pos);
            }
        }
        assert!(trace.len() < 64 * 1024 * 1024);
    }
    panic!("demux work did not terminate")
}
const SCHEMA: &[stream::ElementSpec] = &[
    stream::ElementSpec {
        id: 0xa0,
        parent: stream::Parent::Root,
        kind: stream::Kind::Master { unknown_size: true },
    },
    stream::ElementSpec {
        id: 0xa1,
        parent: stream::Parent::Element(0xa0),
        kind: stream::Kind::Master { unknown_size: true },
    },
    stream::ElementSpec {
        id: 0xa2,
        parent: stream::Parent::Element(0xa1),
        kind: stream::Kind::Metadata,
    },
    stream::ElementSpec {
        id: 0xa3,
        parent: stream::Parent::Element(0xa1),
        kind: stream::Kind::Payload,
    },
    stream::ElementSpec {
        id: 0xa4,
        parent: stream::Parent::Element(0xa0),
        kind: stream::Kind::Metadata,
    },
    stream::ElementSpec {
        id: 0xa5,
        parent: stream::Parent::Element(0xa0),
        kind: stream::Kind::Skip,
    },
    stream::ElementSpec {
        id: 0xec,
        parent: stream::Parent::Global,
        kind: stream::Kind::Skip,
    },
];
fn stream<R: Read>(source: R) -> (Vec<u8>, String, u64) {
    let mut r =
        stream::Reader::new(source, stream::Schema::new(SCHEMA).unwrap(), framing()).unwrap();
    let mut trace = Vec::new();
    let mut buffer = [0; 17];
    for _ in 0..100_000 {
        match r.poll(&mut buffer) {
            Ok(stream::Event::Start { header, kind }) => {
                trace.push(1);
                trace.extend(format!("{header:?}{kind:?}").bytes());
            }
            Ok(stream::Event::End { header, reason }) => {
                trace.push(2);
                trace.extend(format!("{header:?}{reason:?}").bytes());
            }
            Ok(stream::Event::Data { len }) => trace.extend(&buffer[..len]),
            Ok(stream::Event::NeedInput | stream::Event::Yield) => (),
            Ok(stream::Event::SourceEof) => return (trace, "eof".into(), r.position()),
            Err(e) => {
                let pos = r.position();
                assert!(matches!(
                    r.poll(&mut buffer),
                    Err(stream::StreamError::Failed)
                ));
                assert_eq!(r.position(), pos);
                return (trace, format!("{e:?}"), pos);
            }
        }
        assert!(trace.len() < 64 * 1024 * 1024);
    }
    panic!("stream work did not terminate")
}
struct Ranges<'a> {
    inner: Chunk<'a>,
}
impl ReadAt for Ranges<'_> {
    fn snapshot(&mut self) -> io::Result<Snapshot> {
        Ok(Snapshot {
            length: self.inner.bytes.len() as u64,
            revision: 1,
        })
    }
    fn read_at(&mut self, offset: u64, out: &mut [u8]) -> io::Result<usize> {
        self.inner.position = offset as usize;
        self.inner.read(out)
    }
}
fn seek_limits() -> SeekLimits {
    SeekLimits {
        read: ReadLimits {
            max_calls: 50_000,
            max_bytes: 65536,
            max_extents: 32,
        },
        max_entries: 64,
        max_index_bytes: 8192,
        max_index_elements: 2048,
        max_positions_per_cue: 8,
        max_prepare_steps: 16,
        max_scan_headers: 2048,
    }
}
fn seek<S: ReadAt>(source: S, request: i128) -> (Vec<u8>, String) {
    let mut r =
        SeekReader::new(source, AudioSelection::First, demux_limits(), seek_limits()).unwrap();
    let cancel = AtomicBool::new(false);
    let mut trace = Vec::new();
    let mut prepared = false;
    for _ in 0..5000 {
        match r.prepare(&cancel) {
            Ok(Preparation::Ready) => {
                prepared = true;
                break;
            }
            Ok(Preparation::Yield) => (),
            Err(SeekError::Source(SourceError::NeedRange(_))) => (),
            Err(e) => return (trace, format!("{e:?}")),
        }
    }
    assert!(prepared, "prepare did not terminate");
    'requests: for time in [0, request, 0] {
        let result = loop {
            match r.seek(
                SeekRequest {
                    timestamp_ns: time,
                    preroll_ns: 0,
                },
                &cancel,
            ) {
                Ok(result) => break result,
                Err(SeekError::Source(SourceError::NeedRange(_))) => (),
                Err(SeekError::RequestOverflow) => {
                    trace.extend(b"request_overflow");
                    continue 'requests;
                }
                Err(e) => return (trace, format!("{e:?}")),
            }
        };
        trace.extend(format!("{result:?}").bytes());
        let mut ended = false;
        for _ in 0..100_000 {
            match r.poll(&cancel) {
                Ok(SeekEvent::Packet(p)) => packet(&mut trace, p),
                Ok(SeekEvent::Yield) => (),
                Ok(SeekEvent::End(end)) => {
                    trace.extend(format!("{end:?}").bytes());
                    ended = true;
                    break;
                }
                Err(SeekError::Source(SourceError::NeedRange(_))) => (),
                Err(e) => return (trace, format!("{e:?}")),
            }
            assert!(trace.len() < 64 * 1024 * 1024);
        }
        assert!(ended, "seek suffix did not end");
    }
    (trace, "done".into())
}
fn prepared<S: ReadAt>(source: S, request: i128) -> (Vec<u8>, String) {
    let mut owner =
        PreparedSource::new(source, AudioSelection::First, demux_limits(), seek_limits()).unwrap();
    let cancel = AtomicBool::new(false);
    let mut trace = Vec::new();
    let mut ready = false;
    for _ in 0..5000 {
        match owner.prepare(&cancel) {
            Ok(Preparation::Ready) => {
                ready = true;
                break;
            }
            Ok(Preparation::Yield) | Err(SeekError::Source(SourceError::NeedRange(_))) => (),
            Err(e) => return (trace, format!("{e:?}")),
        }
    }
    assert!(ready, "prepared owner did not terminate");
    'requests: for time in [0, request, 0] {
        let mut opened = false;
        for _ in 0..5000 {
            let mut session = match owner.open(
                SeekRequest {
                    timestamp_ns: time,
                    preroll_ns: 0,
                },
                &cancel,
            ) {
                Ok(session) => session,
                Err(SeekError::Source(SourceError::NeedRange(_))) => continue,
                Err(SeekError::RequestOverflow) => {
                    trace.extend(b"request_overflow");
                    continue 'requests;
                }
                Err(e) => return (trace, format!("{e:?}")),
            };
            trace.extend(format!("{:?}", session.start()).bytes());
            let mut ended = false;
            for _ in 0..100_000 {
                match session.poll(&cancel) {
                    Ok(SeekEvent::Packet(p)) => packet(&mut trace, p),
                    Ok(SeekEvent::Yield) => (),
                    Ok(SeekEvent::End(end)) => {
                        trace.extend(format!("{end:?}").bytes());
                        ended = true;
                        break;
                    }
                    Err(SeekError::Source(SourceError::NeedRange(_))) => (),
                    Err(e) => return (trace, format!("{e:?}")),
                }
                assert!(trace.len() < 64 * 1024 * 1024);
            }
            assert!(ended, "prepared session suffix did not end");
            opened = true;
            break;
        }
        assert!(opened, "prepared session opening did not terminate");
    }
    (trace, "done".into())
}

fn scalars(bytes: &[u8]) {
    for prefix in [bytes, &bytes[..bytes.len().min(12)]] {
        let _ = unlace::parse_id(prefix);
        let _ = unlace::parse_size(prefix);
    }
    for kind in [BlockKind::Block, BlockKind::SimpleBlock] {
        if let Ok(block) = parse_block(bytes, kind, demux_limits().block) {
            let frames: Vec<_> = block.frames().collect();
            assert!(frames.len() <= 32);
            let total: usize = frames.iter().map(|f| f.len()).sum();
            assert!(total <= bytes.len());
            let again = parse_block(bytes, kind, demux_limits().block).unwrap();
            assert_eq!(frames, again.frames().collect::<Vec<_>>());
        }
    }
    let _ = unlace::opus::parse_opus_head(bytes);
}

use std::{cell::RefCell, rc::Rc};
struct Versioned {
    state: Rc<RefCell<(u128, u64, u8)>>,
}
impl ReadAt for Versioned {
    fn snapshot(&mut self) -> io::Result<Snapshot> {
        Ok(Snapshot {
            length: 8,
            revision: self.state.borrow().0,
        })
    }
    fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        let mut state = self.state.borrow_mut();
        state.1 += 1;
        if state.2 == 1 {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        if state.2 == 2 {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if state.2 == 3 {
            return Err(io::Error::other("injected"));
        }
        let n = buffer.len().min(2).min(8 - offset as usize);
        if state.2 == 4 {
            state.0 = 2;
        }
        for (i, b) in buffer[..n].iter_mut().enumerate() {
            *b = offset as u8 + i as u8;
        }
        Ok(n)
    }
}
fn sources(actions: &[u8]) {
    let state = Rc::new(RefCell::new((1, 0, 0)));
    let provider = Versioned {
        state: state.clone(),
    };
    let mut reader = RangeReader::new(
        provider,
        ReadLimits {
            max_bytes: 16,
            max_calls: 8,
            max_extents: 2,
        },
    )
    .unwrap();
    let mut poisoned = false;
    for pair in actions.chunks(2).take(1024) {
        let a = pair[0];
        let b = pair.get(1).copied().unwrap_or(0);
        state.borrow_mut().0 = if a & 16 == 0 { 1 } else { 2 };
        state.borrow_mut().2 = b % 5;
        let before = state.borrow().1;
        let mismatch = state.borrow().0 != 1;
        let result = match a % 3 {
            0 => reader.begin_operation().map(|_| 0),
            1 => reader.check_snapshot().map(|_| 0),
            _ => {
                let mut bytes = [255; 8];
                let offset = u64::from(b % 10);
                let len = (a as usize / 32) % 9;
                let out = reader.read_at(offset, &mut bytes[..len]);
                if let Ok(n) = out {
                    assert!(offset + n as u64 <= 8);
                    for (i, byte) in bytes[..n].iter().enumerate() {
                        assert_eq!(*byte, offset as u8 + i as u8);
                    }
                }
                out
            }
        };
        if poisoned || mismatch {
            assert!(matches!(result, Err(SourceError::SourceChanged)));
            assert_eq!(state.borrow().1, before);
            poisoned = true;
        } else if matches!(result, Err(SourceError::SourceChanged)) {
            assert!(state.borrow().0 != 1);
            poisoned = true;
        }
        assert_eq!(
            reader.snapshot(),
            Snapshot {
                length: 8,
                revision: 1
            }
        );
        assert!(
            reader.report().bytes <= 16
                && reader.report().calls <= 8
                && reader.report().extents.len() <= 2
        );
    }
}
fn main() {
    let mut bytes = Vec::new();
    io::stdin().take(8194).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= 8193);
    let Some((&mode, data)) = bytes.split_first() else {
        return;
    };
    match mode % 5 {
        0 => scalars(data),
        1 => assert!(
            stream(data) == stream(chunk(data)),
            "framing trace mismatch"
        ),
        2 => assert!(demux(data) == demux(chunk(data)), "demux trace mismatch"),
        3 => {
            if data.len() < 16 {
                return;
            }
            let mut timestamp = [0; 16];
            timestamp.copy_from_slice(&data[..16]);
            let data = &data[16..];
            let request = i128::from_le_bytes(timestamp);
            let direct = seek(SeekSource::new(Cursor::new(data), 1), request);
            assert!(
                direct == seek(Ranges { inner: chunk(data) }, request),
                "seek trace mismatch"
            );
            let retained = prepared(SeekSource::new(Cursor::new(data), 1), request);
            assert!(
                retained == prepared(Ranges { inner: chunk(data) }, request),
                "prepared session trace mismatch"
            );
            if direct.1 == "done" && retained.1 == "done" {
                assert!(
                    direct == retained,
                    "prepared/direct successful suffix mismatch"
                );
            }
        }
        _ => sources(data),
    }
}
