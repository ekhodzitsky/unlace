//! Inspect an EBML ID and size without buffering the element payload.
use std::error::Error;
use unlace::{ElementSize, Parsed, parse_id, parse_size};

fn main() -> Result<(), Box<dyn Error>> {
    let header = [0x1a, 0x45, 0xdf, 0xa3, 0xff];
    let Parsed::Complete {
        value: id,
        consumed,
    } = parse_id(&header)?
    else {
        return Err("incomplete element ID".into());
    };
    let size = parse_size(&header[consumed..])?;
    assert_eq!(
        size,
        Parsed::Complete {
            value: ElementSize::Unknown,
            consumed: 1
        }
    );
    println!("id=0x{:08x} size=unknown", id.value());
    Ok(())
}
