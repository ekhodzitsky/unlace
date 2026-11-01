//! Allocation-free Opus identification metadata, without codec decoding.
//!
//! This parses the OpusHead stored in Matroska CodecPrivate. It does not parse
//! audio packets, decode samples, apply gain, resample or remove pre-skip.
//!
//! ```
//! use unlace::opus::parse_opus_head;
//! let bytes = [b'O', b'p', b'u', b's', b'H', b'e', b'a', b'd',
//!     1, 2, 0x38, 0x01, 0x80, 0xbb, 0, 0, 0, 0, 0];
//! let head = parse_opus_head(&bytes)?;
//! assert_eq!(head.channels(), 2);
//! assert_eq!(head.pre_skip_samples(), 312);
//! assert_eq!(head.pre_skip_ns_numerator(), 312_000_000_000);
//! // The exact time is this numerator divided by 48,000, or 6,500,000 ns.
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
use std::fmt;

/// Why an Opus identification header cannot be interpreted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpusErrorKind {
    /// Missing bytes for a required field or channel mapping.
    Truncated,
    /// The eight-byte OpusHead signature does not match.
    Signature,
    /// The major version is incompatible with this parser.
    UnsupportedVersion,
    /// The channel mapping family is not implemented (only 0, 1, 255 are).
    UnsupportedMapping,
    /// Zero channels or a count outside the mapping family's range.
    Channels,
    /// Invalid stream/coupled counts or channel-map index.
    Mapping,
    /// Version 0/1 has bytes after its defined fields.
    TrailingData,
}
/// Header failure at a byte offset relative to the start of CodecPrivate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpusError {
    /// Offset of the invalid field, or first missing byte for truncation.
    pub offset: usize,
    /// Typed failure reason; unsupported versions/families are distinct.
    pub kind: OpusErrorKind,
}
impl fmt::Display for OpusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OpusHead {:?} at byte {}", self.kind, self.offset)
    }
}
impl std::error::Error for OpusError {}

/// Checked, borrowed identification metadata. All construction goes through
/// [`parse_opus_head`]; no decoder capability is implied by successful parsing.
#[derive(Clone, Copy, Debug)]
pub struct OpusHead<'a> {
    raw: &'a [u8],
    version: u8,
    channels: u8,
    pre_skip: u16,
    input_rate: u32,
    gain: i16,
    family: u8,
    streams: u8,
    coupled: u8,
    mapping: &'a [u8],
    extension: &'a [u8],
}
impl<'a> OpusHead<'a> {
    /// The exact original bytes, including compatible extension bytes.
    pub fn raw(self) -> &'a [u8] {
        self.raw
    }
    /// Original version (major zero, minor zero through fifteen).
    pub fn version(self) -> u8 {
        self.version
    }
    /// Output channel count, not the packet's internal encoded channel count.
    pub fn channels(self) -> u8 {
        self.channels
    }
    /// Decoder samples per channel to discard once at stream start, at 48 kHz.
    pub fn pre_skip_samples(self) -> u16 {
        self.pre_skip
    }
    /// Informational original input frequency in hertz; zero means unspecified.
    /// This is not an instruction to select a decoder output frequency.
    pub fn input_sample_rate(self) -> u32 {
        self.input_rate
    }
    /// Signed output gain in units of 1/256 dB. Not applied by this library.
    pub fn output_gain_q8_db(self) -> i16 {
        self.gain
    }
    /// Validated mapping family: 0, 1 or 255. No downmix is performed.
    pub fn mapping_family(self) -> u8 {
        self.family
    }
    /// Number of encoded Opus streams, including coupled streams.
    pub fn stream_count(self) -> u8 {
        self.streams
    }
    /// Number of streams requiring stereo decoder output.
    pub fn coupled_stream_count(self) -> u8 {
        self.coupled
    }
    /// Explicit map for families 1/255. Family 0 returns an empty slice; its
    /// implicit map is mono `[0]` or stereo `[0,1]`. Index 255 denotes silence.
    pub fn channel_mapping(self) -> &'a [u8] {
        self.mapping
    }
    /// Additional bytes permitted by a compatible minor version greater than 1.
    pub fn extension(self) -> &'a [u8] {
        self.extension
    }
    /// Exact pre-skip time as a numerator of nanoseconds over 48,000.
    /// Retaining the denominator avoids losing fractional nanoseconds.
    pub fn pre_skip_ns_numerator(self) -> u64 {
        u64::from(self.pre_skip) * 1_000_000_000
    }
}

/// Parse RFC 7845 identification fields and channel maps, borrowing all bytes.
///
/// Families 0, 1 and 255 are supported. Other families, including newer defined
/// mappings requiring different structures, fail explicitly. Versions below 16
/// have compatible major version zero. For minor versions above 1, additional
/// bytes are retained rather than rejected or silently discarded.
pub fn parse_opus_head(input: &[u8]) -> Result<OpusHead<'_>, OpusError> {
    let error = |offset, kind| OpusError { offset, kind };
    for (offset, &expected) in b"OpusHead".iter().enumerate() {
        match input.get(offset) {
            None => return Err(error(offset, OpusErrorKind::Truncated)),
            Some(&actual) if actual != expected => {
                return Err(error(offset, OpusErrorKind::Signature));
            }
            _ => (),
        }
    }
    if input.len() < 19 {
        return Err(error(input.len(), OpusErrorKind::Truncated));
    }
    let version = input[8];
    if version >= 16 {
        return Err(error(8, OpusErrorKind::UnsupportedVersion));
    }
    let channels = input[9];
    let family = input[18];
    if channels == 0 || (family == 0 && channels > 2) || (family == 1 && channels > 8) {
        return Err(error(9, OpusErrorKind::Channels));
    }
    let (streams, coupled, mapping, end) = match family {
        0 => (1, channels - 1, &input[19..19], 19),
        1 | 255 => {
            let end = 21 + usize::from(channels);
            if input.len() < end {
                return Err(error(input.len(), OpusErrorKind::Truncated));
            }
            let streams = input[19];
            let coupled = input[20];
            let decoded = u16::from(streams) + u16::from(coupled);
            if streams == 0 {
                return Err(error(19, OpusErrorKind::Mapping));
            }
            if coupled > streams || decoded > 255 {
                return Err(error(20, OpusErrorKind::Mapping));
            }
            for (index, &value) in input[21..end].iter().enumerate() {
                if value != 255 && u16::from(value) >= decoded {
                    return Err(error(21 + index, OpusErrorKind::Mapping));
                }
            }
            (streams, coupled, &input[21..end], end)
        }
        _ => return Err(error(18, OpusErrorKind::UnsupportedMapping)),
    };
    if version <= 1 && input.len() != end {
        return Err(error(end, OpusErrorKind::TrailingData));
    }
    Ok(OpusHead {
        raw: input,
        version,
        channels,
        family,
        streams,
        coupled,
        mapping,
        extension: &input[end..],
        pre_skip: u16::from_le_bytes([input[10], input[11]]),
        input_rate: u32::from_le_bytes([input[12], input[13], input[14], input[15]]),
        gain: i16::from_le_bytes([input[16], input[17]]),
    })
}
