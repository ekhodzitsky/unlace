//! Bounded, allocation-free Block/SimpleBlock payload parsing.
//!
//! Input must be the complete payload of one finite container element. Framing
//! and source truncation belong to [`crate::stream`]. This module does not infer
//! per-frame timestamps from codec bytes or apply delay and discard padding.
//!
//! ```
//! use unlace::block::{parse_block, BlockKind, BlockLimits};
//! // Track 1, relative timestamp -2, keyframe, one encoded frame.
//! let bytes = [0x81, 0xff, 0xfe, 0x80, 1, 2, 3];
//! let block = parse_block(&bytes, BlockKind::SimpleBlock, BlockLimits::default())?;
//! assert_eq!(block.track_number(), 1);
//! assert_eq!(block.relative_timestamp(), -2);
//! assert_eq!(block.frames().collect::<Vec<_>>(), [&[1, 2, 3][..]]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::{Parsed, vint};
use std::fmt;

/// The container element carrying these bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    /// A SimpleBlock has keyframe and discardable flags.
    SimpleBlock,
    /// A Block inside BlockGroup derives keyframe status from group metadata.
    Block,
}

/// How frame lengths are represented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lacing {
    /// One frame, without a lace header.
    None,
    /// Lengths expressed as sums of bytes, with 255 extending the sum.
    Xiph,
    /// Every frame has the same length.
    Fixed,
    /// First length is unsigned VINT; subsequent length differences are signed.
    Ebml,
}

/// Independent limits, enforced before constructing any frame views.
#[derive(Clone, Copy, Debug)]
pub struct BlockLimits {
    /// Maximum complete block payload size in bytes.
    pub max_block_size: usize,
    /// Maximum single encoded frame size in bytes.
    pub max_frame_size: usize,
    /// Maximum frames per block, from 1 through the format maximum of 256.
    pub max_frames: usize,
    /// Maximum lace header bytes, including the count byte. Bounds Xiph work
    /// even when a hostile input contains a long run of 255 bytes.
    pub max_lace_header_size: usize,
}
impl Default for BlockLimits {
    fn default() -> Self {
        Self {
            max_block_size: 16 * 1024 * 1024,
            max_frame_size: 16 * 1024 * 1024,
            max_frames: 256,
            max_lace_header_size: 64 * 1024,
        }
    }
}

/// A structural block parsing failure, without codec interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockError {
    /// Missing track, timestamp, flags, count or lace length bytes.
    IncompleteHeader,
    /// A VINT has no marker.
    InvalidVint,
    /// Track number zero is invalid.
    ZeroTrack,
    /// Reserved flags are nonzero.
    ReservedFlags,
    /// Lacing was used for a single frame.
    SingleFrameLace,
    /// Length arithmetic is invalid or extends beyond the complete payload.
    InvalidFrameSize,
    /// Fixed-lace data length is not divisible by the frame count.
    UnequalFixedFrames,
    /// An explicit block, frame, frame-count or lace-work limit was exceeded.
    Limit(&'static str),
    /// Configured maximum frame count is outside 1 through 256.
    Configuration,
}
impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompleteHeader => f.write_str("incomplete block header"),
            Self::InvalidVint => f.write_str("invalid block VINT"),
            Self::ZeroTrack => f.write_str("zero block track number"),
            Self::ReservedFlags => f.write_str("nonzero reserved block flags"),
            Self::SingleFrameLace => f.write_str("lacing cannot contain a single frame"),
            Self::InvalidFrameSize => f.write_str("invalid block frame size"),
            Self::UnequalFixedFrames => f.write_str("fixed lace has unequal frame sizes"),
            Self::Limit(kind) => write!(f, "block resource limit: {kind}"),
            Self::Configuration => f.write_str("invalid block limits"),
        }
    }
}
impl std::error::Error for BlockError {}

/// Validated views into the original complete encoded block payload.
///
/// No frame bytes are copied or transformed. Private frame lengths are checked
/// once against the data slice; construction cannot expose invalid ranges.
#[derive(Debug)]
pub struct Block<'a> {
    track_number: u64,
    timestamp: i16,
    flags: u8,
    kind: BlockKind,
    lacing: Lacing,
    data: &'a [u8],
    frame_sizes: [usize; 256],
    frame_count: usize,
}
impl<'a> Block<'a> {
    /// Nonzero track number, with the VINT marker removed.
    pub fn track_number(&self) -> u64 {
        self.track_number
    }

    /// Signed relative timestamp in Track Ticks for the first frame. Later
    /// laced-frame timestamps require track durations or codec information;
    /// assigning this same timestamp to every frame would lose information.
    pub fn relative_timestamp(&self) -> i16 {
        self.timestamp
    }

    /// Original flags, including unused bits.
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// Lacing representation used in the original block.
    pub fn lacing(&self) -> Lacing {
        self.lacing
    }

    /// SimpleBlock keyframe flag, or None for BlockGroup's Block.
    pub fn keyframe(&self) -> Option<bool> {
        (self.kind == BlockKind::SimpleBlock).then_some(self.flags & 0x80 != 0)
    }

    /// Whether the codec should decode these frames without displaying them.
    pub fn invisible(&self) -> bool {
        self.flags & 0x08 != 0
    }

    /// SimpleBlock discardable flag, or None for BlockGroup's Block.
    pub fn discardable(&self) -> Option<bool> {
        (self.kind == BlockKind::SimpleBlock).then_some(self.flags & 1 != 0)
    }

    /// Encoded frames in their original order, including any empty frames.
    /// Each slice borrows the original input for its original lifetime.
    pub fn frames(&self) -> impl ExactSizeIterator<Item = &'a [u8]> + '_ {
        let mut rest = self.data;
        self.frame_sizes[..self.frame_count]
            .iter()
            .map(move |&size| {
                // parse_block checked that all lengths partition data exactly.
                let (frame, tail) = rest.split_at(size);
                rest = tail;
                frame
            })
    }
}

/// Parse one complete Block or SimpleBlock payload under explicit limits.
///
/// The first frame timestamp is preserved as a signed relative value; container
/// timestamp scaling and track selection are separate operations. Empty frame
/// data is structurally permitted and is left for codec-specific validation.
/// Parsing uses fixed storage for at most 256 lengths, with no allocations.
pub fn parse_block(
    input: &[u8],
    kind: BlockKind,
    limits: BlockLimits,
) -> Result<Block<'_>, BlockError> {
    if !(1..=256).contains(&limits.max_frames) {
        return Err(BlockError::Configuration);
    }
    if input.len() > limits.max_block_size {
        return Err(BlockError::Limit("block size"));
    }
    let mut cursor = 0;
    let (track_number, _) = unsigned(input, &mut cursor)?;
    if track_number == 0 {
        return Err(BlockError::ZeroTrack);
    }
    let header = input
        .get(cursor..cursor + 3)
        .ok_or(BlockError::IncompleteHeader)?;
    let timestamp = i16::from_be_bytes([header[0], header[1]]);
    let flags = header[2];
    cursor += 3;
    let reserved = if kind == BlockKind::SimpleBlock {
        0x70
    } else {
        0xf0
    };
    if flags & reserved != 0 {
        return Err(BlockError::ReservedFlags);
    }
    let lacing = match flags & 6 {
        0 => Lacing::None,
        2 => Lacing::Xiph,
        4 => Lacing::Fixed,
        _ => Lacing::Ebml,
    };
    let lace_start = cursor;
    let mut frame_sizes = [0usize; 256];
    let frame_count = if lacing == Lacing::None {
        1
    } else {
        let count = usize::from(lace_byte(input, &mut cursor, lace_start, limits)?) + 1;
        if count == 1 {
            return Err(BlockError::SingleFrameLace);
        }
        if count > limits.max_frames {
            return Err(BlockError::Limit("frame count"));
        }
        count
    };
    match lacing {
        Lacing::None => frame_sizes[0] = input.len() - cursor,
        Lacing::Fixed => {
            let data_size = input.len() - cursor;
            if data_size % frame_count != 0 {
                return Err(BlockError::UnequalFixedFrames);
            }
            frame_sizes[..frame_count].fill(data_size / frame_count);
        }
        Lacing::Xiph => {
            for size in &mut frame_sizes[..frame_count - 1] {
                loop {
                    let byte = lace_byte(input, &mut cursor, lace_start, limits)?;
                    *size = size
                        .checked_add(usize::from(byte))
                        .ok_or(BlockError::InvalidFrameSize)?;
                    if *size > limits.max_frame_size {
                        return Err(BlockError::Limit("frame size"));
                    }
                    if byte != 255 {
                        break;
                    }
                }
            }
        }
        Lacing::Ebml => {
            frame_sizes[0] =
                usize::try_from(lace_unsigned(input, &mut cursor, lace_start, limits)?.0)
                    .map_err(|_| BlockError::InvalidFrameSize)?;
            for index in 1..frame_count - 1 {
                let (raw, width) = lace_unsigned(input, &mut cursor, lace_start, limits)?;
                let bias = (1i64 << (7 * width - 1)) - 1;
                // Eight-byte VINTs fit 56 bits, including the full unsigned
                // all-one value: size sentinels do not apply to lace integers.
                let delta = raw as i64 - bias;
                let size = frame_sizes[index - 1] as i128 + i128::from(delta);
                frame_sizes[index] =
                    usize::try_from(size).map_err(|_| BlockError::InvalidFrameSize)?;
            }
        }
    }
    let data = &input[cursor..];
    if matches!(lacing, Lacing::Xiph | Lacing::Ebml) {
        let mut remaining = data.len();
        for &size in &frame_sizes[..frame_count - 1] {
            remaining = remaining
                .checked_sub(size)
                .ok_or(BlockError::InvalidFrameSize)?;
        }
        frame_sizes[frame_count - 1] = remaining;
    }
    if frame_sizes[..frame_count]
        .iter()
        .any(|&size| size > limits.max_frame_size)
    {
        return Err(BlockError::Limit("frame size"));
    }
    Ok(Block {
        track_number,
        timestamp,
        flags,
        kind,
        lacing,
        data,
        frame_sizes,
        frame_count,
    })
}

fn unsigned(input: &[u8], cursor: &mut usize) -> Result<(u64, usize), BlockError> {
    match vint(&input[*cursor..], 8).map_err(|_| BlockError::InvalidVint)? {
        Parsed::NeedMore { .. } => Err(BlockError::IncompleteHeader),
        Parsed::Complete { value, consumed } => {
            *cursor += consumed;
            Ok((value, consumed))
        }
    }
}

fn lace_byte(
    input: &[u8],
    cursor: &mut usize,
    start: usize,
    limits: BlockLimits,
) -> Result<u8, BlockError> {
    if *cursor - start >= limits.max_lace_header_size {
        return Err(BlockError::Limit("lace header size"));
    }
    let byte = *input.get(*cursor).ok_or(BlockError::IncompleteHeader)?;
    *cursor += 1;
    Ok(byte)
}

fn lace_unsigned(
    input: &[u8],
    cursor: &mut usize,
    start: usize,
    limits: BlockLimits,
) -> Result<(u64, usize), BlockError> {
    let first = *input.get(*cursor).ok_or(BlockError::IncompleteHeader)?;
    if first == 0 {
        return Err(BlockError::InvalidVint);
    }
    let width = first.leading_zeros() as usize + 1;
    if width > limits.max_lace_header_size.saturating_sub(*cursor - start) {
        return Err(BlockError::Limit("lace header size"));
    }
    unsigned(input, cursor)
}
