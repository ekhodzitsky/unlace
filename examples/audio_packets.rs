//! Emit a packet manifest from sequential input, optionally selecting TrackNumber.
//! Raw packet hex is for correctness comparison, not a performance benchmark.
use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Write},
};
use unlace::demux::{AudioSelection, Completion, DemuxEvent, DemuxLimits, Demuxer};

fn hex(output: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    for byte in bytes {
        write!(output, "{byte:02x}")?;
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: audio_packets INPUT [TRACK_NUMBER]")?;
    let selection = match args.next() {
        Some(number) => AudioSelection::Number(number.parse()?),
        None => AudioSelection::First,
    };
    if args.next().is_some() {
        return Err("usage: audio_packets INPUT [TRACK_NUMBER]".into());
    }
    let mut demuxer = Demuxer::new(
        BufReader::new(File::open(path)?),
        selection,
        DemuxLimits::default(),
    )?;
    let mut output = BufWriter::new(io::stdout().lock());
    loop {
        match demuxer.poll()? {
            DemuxEvent::TracksReady => {
                for track in demuxer.tracks() {
                    write!(
                        output,
                        "{{\"event\":\"track\",\"number\":{},\"uid\":{},\"type\":{},\"codec_id_hex\":\"",
                        track.number, track.uid, track.track_type
                    )?;
                    hex(&mut output, track.codec_id.as_bytes())?;
                    write!(output, "\",\"codec_private_hex\":\"")?;
                    hex(&mut output, &track.codec_private)?;
                    writeln!(
                        output,
                        "\",\"codec_delay_ns\":{},\"seek_preroll_ns\":{},\"max_block_addition_id\":{},\"opus_pre_skip_samples\":{}}}",
                        track.codec_delay_ns,
                        track.seek_preroll_ns,
                        track.max_block_addition_id,
                        track.opus_timing.map_or_else(
                            || "null".to_owned(),
                            |value| value.pre_skip_samples().to_string()
                        )
                    )?;
                }
            }
            DemuxEvent::Packet(packet) => {
                write!(
                    output,
                    "{{\"event\":\"packet\",\"track\":{},\"timestamp_ns\":",
                    packet.track_number
                )?;
                match packet.timestamp_ns {
                    Some(value) => write!(output, "{value}")?,
                    None => output.write_all(b"null")?,
                }
                write!(
                    output,
                    ",\"block_timestamp_ns\":{},\"lace_index\":{},\"lace_count\":{},\"discard_padding_ns\":{},\"block_discard_padding_ns\":{},\"data_hex\":\"",
                    packet.block_timestamp_ns,
                    packet.lace_index,
                    packet.lace_count,
                    packet.discard_padding_ns,
                    packet.block_discard_padding_ns
                )?;
                hex(&mut output, packet.data)?;
                writeln!(output, "\"}}")?;
            }
            DemuxEvent::NeedInput => {
                return Err("file input unexpectedly returned WouldBlock".into());
            }
            DemuxEvent::Yield => (),
            DemuxEvent::End(completion) => {
                let value = match completion {
                    Completion::FiniteSegment => "finite_segment",
                    Completion::UnknownSizeSegment => "unknown_size_segment",
                };
                writeln!(output, "{{\"event\":\"end\",\"completion\":\"{value}\"}}")?;
                output.flush()?;
                break;
            }
        }
    }
    Ok(())
}
