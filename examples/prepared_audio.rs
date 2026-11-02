//! Retain one original provider across exclusive consumer sessions.
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    sync::atomic::AtomicBool,
    time::Instant,
};
use unlace::{
    demux::{AudioSelection, DemuxLimits},
    seek::{Preparation, PreparedSource, SeekEvent, SeekLimits, SeekRequest},
    source::SeekSource,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: prepared_audio INPUT TRACK_NUMBER TIMESTAMP_NS...")?;
    let track = args.next().ok_or("missing track number")?.parse()?;
    let mut requests = Vec::new();
    for value in args {
        if requests.len() == 32 {
            return Err("example request limit is 32".into());
        }
        requests.push(value.parse::<i128>()?);
    }
    if requests.is_empty() {
        return Err("at least one timestamp is required".into());
    }
    // Keep the original File immutable for the entire owner's lifetime. Generic
    // Read+Seek cannot detect same-length overwrites; revision 1 is not a file ID.
    let source = SeekSource::new(File::open(path)?, 1);
    let cancel = AtomicBool::new(false);
    let start = Instant::now();
    let mut owner = PreparedSource::new(
        source,
        AudioSelection::Number(track),
        DemuxLimits::default(),
        SeekLimits::default(),
    )?;
    let (mut calls, mut bytes) = (0u128, 0u128);
    loop {
        let status = owner.prepare(&cancel)?;
        calls += u128::from(owner.read_report().calls);
        bytes += u128::from(owner.read_report().bytes);
        if status == Preparation::Ready {
            break;
        }
    }
    let mut output = BufWriter::new(io::stdout().lock());
    writeln!(
        output,
        "prepare_ns={} read_calls={calls} read_bytes={bytes}",
        start.elapsed().as_nanos()
    )?;
    for timestamp_ns in requests {
        let mut session = owner.open(
            SeekRequest {
                timestamp_ns,
                preroll_ns: 0,
            },
            &cancel,
        )?;
        writeln!(
            output,
            "session={:?} open_calls={} open_bytes={}",
            session.start(),
            session.read_report().calls,
            session.read_report().bytes
        )?;
        loop {
            match session.poll(&cancel)? {
                SeekEvent::Packet(p) => writeln!(
                    output,
                    "track={} offset={} timestamp_ns={:?} bytes={} lace={}/{}",
                    p.track_number,
                    p.block_offset,
                    p.timestamp_ns,
                    p.data.len(),
                    p.lace_index,
                    p.lace_count
                )?,
                SeekEvent::Yield => (),
                SeekEvent::End(end) => {
                    writeln!(output, "suffix_end={end:?}")?;
                    break;
                }
            }
        }
    }
    output.flush()?;
    Ok(())
}
