//! Sequential framing of a synthetic EBML header, without Seek or payload storage.
use unlace::stream::{ElementSpec, Event, Kind, Limits, Parent, Reader, Schema};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entries = [
        ElementSpec {
            id: 0x1a45dfa3,
            parent: Parent::Root,
            kind: Kind::Master {
                unknown_size: false,
            },
        },
        ElementSpec {
            id: 0x4286,
            parent: Parent::Element(0x1a45dfa3),
            kind: Kind::Metadata,
        },
    ];
    // EBML master containing EBMLVersion=1. This example frames bytes only;
    // it does not check the semantic requirements for a complete document.
    let input = [0x1a, 0x45, 0xdf, 0xa3, 0x84, 0x42, 0x86, 0x81, 1];
    let mut reader = Reader::new(input.as_slice(), Schema::new(&entries)?, Limits::default())?;
    let mut buffer = [0; 128];
    loop {
        match reader.poll(&mut buffer)? {
            Event::Start { header, .. } => {
                println!("start {:#x} at {}", header.id.value(), header.offset)
            }
            Event::Data { len } => println!("data {:?}", &buffer[..len]),
            Event::End { header, reason } => println!("end {:#x}: {reason:?}", header.id.value()),
            Event::NeedInput => return Err("this in-memory source cannot block".into()),
            Event::Yield => (),
            Event::SourceEof => break,
        }
    }
    Ok(())
}
