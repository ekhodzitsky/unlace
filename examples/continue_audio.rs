//! Resume the same demuxer after simulated producer pauses between tiny chunks.
use std::{
    cell::Cell,
    fs::File,
    io::{self, BufReader, Read},
    rc::Rc,
};
use unlace::demux::{AudioSelection, DemuxEvent, DemuxLimits, Demuxer};

struct Paused<R> {
    source: R,
    ready: Rc<Cell<bool>>,
}
impl<R: Read> Read for Paused<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        if !self.ready.get() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let limit = output.len().min(17);
        let count = self.source.read(&mut output[..limit])?;
        if count != 0 {
            self.ready.set(false);
        }
        // Zero means final EOF, never a temporary producer pause.
        Ok(count)
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: continue_audio INPUT [TRACK_NUMBER]")?;
    let selection = match args.next() {
        Some(number) => AudioSelection::Number(number.parse()?),
        None => AudioSelection::First,
    };
    if args.next().is_some() {
        return Err("usage: continue_audio INPUT [TRACK_NUMBER]".into());
    }
    let ready = Rc::new(Cell::new(false));
    let source = Paused {
        source: BufReader::new(File::open(path)?),
        ready: Rc::clone(&ready),
    };
    let mut reader = Demuxer::new(source, selection, DemuxLimits::default())?;
    let mut packets = 0u64;
    let mut bytes = 0u64;
    let mut pauses = 0u64;
    loop {
        match reader.poll()? {
            DemuxEvent::TracksReady => println!("tracks={}", reader.tracks().len()),
            DemuxEvent::Packet(packet) => {
                // Packet bytes borrow the reader until the next mutable call.
                // Process them here, or copy under an application-owned limit.
                packets = packets.checked_add(1).ok_or("packet count overflow")?;
                bytes = bytes
                    .checked_add(packet.data.len() as u64)
                    .ok_or("byte count overflow")?;
            }
            DemuxEvent::NeedInput => {
                pauses = pauses.checked_add(1).ok_or("pause count overflow")?;
                // This demonstration immediately grants the next tiny chunk.
                // A real producer waits for availability outside unlace, then
                // resumes this same reader. Do not reset it or substitute EOF.
                ready.set(true);
            }
            DemuxEvent::Yield => {
                // A real event loop may schedule other work before polling again.
            }
            DemuxEvent::End(completion) => {
                println!(
                    "packets={packets} bytes={bytes} pauses={pauses} completion={completion:?}"
                );
                break;
            }
        }
    }
    Ok(())
}
