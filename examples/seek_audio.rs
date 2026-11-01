//! Seek selected audio and read the actual suffix with explicit read accounting.
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    sync::atomic::AtomicBool,
};
use unlace::{
    demux::{AudioSelection, DemuxLimits},
    seek::{Preparation, SeekEvent, SeekLimits, SeekReader, SeekRequest},
    source::SeekSource,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: seek_audio INPUT TIMESTAMP_NS [TRACK_NUMBER]")?;
    let timestamp_ns = args
        .next()
        .ok_or("missing timestamp in signed nanoseconds")?
        .parse()?;
    let selection = match args.next() {
        Some(number) => AudioSelection::Number(number.parse()?),
        None => AudioSelection::First,
    };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    // Keep this owned input immutable. Generic File Read+Seek cannot detect
    // same-length external overwrites. Use a version-aware ReadAt cache for that.
    let source = SeekSource::new(File::open(path)?, 1);
    let mut reader = SeekReader::new(
        source,
        selection,
        DemuxLimits::default(),
        SeekLimits::default(),
    )?;
    let cancel = AtomicBool::new(false);
    let mut open_bytes = 0u128;
    let mut open_calls = 0u128;
    loop {
        let status = reader.prepare(&cancel)?;
        open_bytes += u128::from(reader.read_report().bytes);
        open_calls += u128::from(reader.read_report().calls);
        if status == Preparation::Ready {
            break;
        }
    }
    let result = reader.seek(
        SeekRequest {
            timestamp_ns,
            preroll_ns: 0,
        },
        &cancel,
    )?;
    let mut output = BufWriter::new(io::stdout().lock());
    writeln!(
        output,
        "prepare_bytes={open_bytes} prepare_read_calls={open_calls}"
    )?;
    writeln!(
        output,
        "seek={result:?} read_bytes={} read_calls={}",
        reader.read_report().bytes,
        reader.read_report().calls
    )?;
    loop {
        match reader.poll(&cancel)? {
            SeekEvent::Packet(packet) => writeln!(
                output,
                "track={} block_offset={} timestamp_ns={:?} lace={}/{} bytes={} block_discard_padding_ns={}",
                packet.track_number,
                packet.block_offset,
                packet.timestamp_ns,
                packet.lace_index,
                packet.lace_count,
                packet.data.len(),
                packet.block_discard_padding_ns
            )?,
            SeekEvent::Yield => (),
            SeekEvent::End(completion) => {
                writeln!(output, "suffix_end={completion:?}")?;
                break;
            }
        }
    }
    output.flush()?;
    Ok(())
}
