//! Bounded WebM/Matroska parsing foundations.
//!
//! Provides allocation-free EBML integer parsing, schema-driven incremental
//! framing in [`stream`] and individual Block/SimpleBlock parsing in [`block`].
//! Sequential selected-audio packet events are available in [`demux`]. The
//! supported subset remains under qualification. [`seek`] adds bounded verified
//! container access over immutable positional sources. [`mod@write`] provides explicit
//! live and indexed output from already encoded blocks.
//!
//! ```
//! use unlace::{parse_id, parse_size, ElementSize, Parsed};
//! let Parsed::Complete { value: id, consumed } = parse_id(&[0x1a, 0x45, 0xdf, 0xa3])?
//! else { return Err("incomplete ID".into()); };
//! assert_eq!(id.value(), 0x1a45dfa3);
//! assert_eq!(consumed, 4);
//! assert_eq!(parse_size(&[0xff])?, Parsed::Complete {
//!     value: ElementSize::Unknown, consumed: 1,
//! });
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::fmt;

pub mod stream;

pub mod block;

pub mod demux;

pub mod opus;

pub mod source;

pub mod seek;

pub mod write;

/// A complete value or an incomplete prefix. Input is never mutated or retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parsed<T> {
    /// The value and the number of bytes belonging to this integer.
    Complete {
        /// Parsed value.
        value: T,
        /// Encoded width; trailing input belongs to the caller.
        consumed: usize,
    },
    /// At least this many total bytes are required, including the supplied prefix.
    NeedMore {
        /// Minimum total input length to determine this integer.
        total: usize,
    },
}

/// Validated element ID, preserving its width marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementId(u32);
impl ElementId {
    /// The encoded ID, suitable for comparison with Matroska schema constants.
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Payload size syntax. Unknown size still requires schema validation by a reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementSize {
    /// Finite payload length in bytes; this does not authorize an allocation.
    Known(u64),
    /// All VINT data bits are one. Only eligible master elements may use this.
    Unknown,
}

/// Invalid EBML integer syntax or an unsupported encoded width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// The first byte contains no VINT width marker.
    InvalidMarker,
    /// The marker requests more bytes than this Matroska-oriented API allows.
    WidthExceeded {
        /// Width declared by the marker.
        width: u8,
        /// Maximum width for this integer kind.
        maximum: u8,
    },
    /// An element ID uses the reserved all-one data value.
    ReservedElementId,
    /// An element ID is not the shortest valid representation of its data value.
    NonCanonicalElementId,
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMarker => f.write_str("EBML integer has no width marker"),
            Self::WidthExceeded { width, maximum } => {
                write!(f, "EBML integer width {width} exceeds {maximum}")
            }
            Self::ReservedElementId => f.write_str("reserved EBML element ID"),
            Self::NonCanonicalElementId => f.write_str("noncanonical EBML element ID"),
        }
    }
}
impl std::error::Error for ParseError {}

/// Parse a canonical one-to-four-byte element ID.
///
/// RFC 9559 section 4.2 permits the one-byte ID `0x80`, updating RFC 8794.
/// No schema membership or parent/child relationship is established here.
pub fn parse_id(input: &[u8]) -> Result<Parsed<ElementId>, ParseError> {
    match vint(input, 4)? {
        Parsed::NeedMore { total } => Ok(Parsed::NeedMore { total }),
        Parsed::Complete { value, consumed } => {
            let sentinel = (1u64 << (7 * consumed)) - 1;
            if value == sentinel {
                return Err(ParseError::ReservedElementId);
            }
            if consumed > 1 && value < (1u64 << (7 * (consumed - 1))) - 1 {
                return Err(ParseError::NonCanonicalElementId);
            }
            // vint capped the width at four bytes, so these shifts fit u32.
            let encoded = input
                .iter()
                .take(consumed)
                .fold(0u32, |n, byte| (n << 8) | u32::from(*byte));
            Ok(Parsed::Complete {
                value: ElementId(encoded),
                consumed,
            })
        }
    }
}

/// Parse a one-to-eight-byte payload size, preserving unknown-size sentinels.
///
/// Nonminimal size encodings and zero are legal. Document/schema restrictions
/// such as EBMLMaxSizeLength and payload budgets remain the reader's job.
pub fn parse_size(input: &[u8]) -> Result<Parsed<ElementSize>, ParseError> {
    match vint(input, 8)? {
        Parsed::NeedMore { total } => Ok(Parsed::NeedMore { total }),
        Parsed::Complete { value, consumed } => Ok(Parsed::Complete {
            value: if value == (1u64 << (7 * consumed)) - 1 {
                ElementSize::Unknown
            } else {
                ElementSize::Known(value)
            },
            consumed,
        }),
    }
}

fn vint(input: &[u8], maximum: u8) -> Result<Parsed<u64>, ParseError> {
    let Some(&first) = input.first() else {
        return Ok(Parsed::NeedMore { total: 1 });
    };
    if first == 0 {
        return Err(ParseError::InvalidMarker);
    }
    let width = first.leading_zeros() as u8 + 1;
    if width > maximum {
        return Err(ParseError::WidthExceeded { width, maximum });
    }
    let consumed = usize::from(width);
    if input.len() < consumed {
        return Ok(Parsed::NeedMore { total: consumed });
    }
    let mask = (1u8 << (8 - width)) - 1;
    let mut value = u64::from(first & mask);
    for byte in input.iter().take(consumed).skip(1) {
        value = (value << 8) | u64::from(*byte);
    }
    Ok(Parsed::Complete { value, consumed })
}
