use super::{DemuxError, Track};
use crate::{
    opus::{self, OpusErrorKind},
    stream::Header,
};
use std::fmt;

/// Precision of the container's integer CodecDelay relative to exact pre-skip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelayPrecision {
    /// Exactly equal to the header pre-skip time.
    Exact,
    /// Integer nanosecond representation differs by less than one nanosecond.
    /// Both adjacent integers are accepted because the mapping formula does
    /// not prescribe an integer rounding direction. The raw value is retained.
    Quantized {
        /// Container minus exact header time, in units of 1/48,000 nanoseconds.
        difference: i32,
    },
}
/// Checked relation between exact Opus pre-skip and raw CodecDelay.
#[derive(Clone, Copy, Debug)]
pub struct OpusTiming {
    pre_skip: u16,
    precision: DelayPrecision,
}
impl OpusTiming {
    /// Exact initial decoder sample skip per channel at 48 kHz. Apply once,
    /// without converting the already delay-adjusted packet timestamp again.
    pub fn pre_skip_samples(self) -> u16 {
        self.pre_skip
    }
    /// Exact pre-skip nanosecond numerator over denominator 48,000.
    pub fn pre_skip_ns_numerator(self) -> u64 {
        u64::from(self.pre_skip) * 1_000_000_000
    }
    /// Whether raw integer CodecDelay equals or quantizes exact sample time.
    pub fn delay_precision(self) -> DelayPrecision {
        self.precision
    }
}
/// Typed malformed, conflicting or unsupported timing/initialization metadata.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimingErrorKind {
    /// Invalid scalar width/encoding, sign/range or nonfinite value.
    InvalidScalar,
    /// A singleton timing field occurred more than once.
    DuplicateElement,
    /// An Opus track has no CodecPrivate identification header.
    MissingOpusHead,
    /// An Opus track has no Audio parameters master.
    MissingAudio,
    /// Header syntax, channel-map or unsupported-version/family error.
    OpusHead(OpusErrorKind),
    /// A_OPUS was assigned to a nonaudio TrackType.
    OpusTrackType,
    /// Audio Channels differs from OpusHead's output channel count.
    ChannelsMismatch,
    /// SamplingFrequency differs from OpusHead's input frequency.
    InputRateMismatch,
    /// CodecDelay is not either adjacent integer to exact pre-skip time.
    CodecDelayMismatch,
    /// Checked timestamp or duration arithmetic exceeds i128.
    ArithmeticOverflow,
    /// Padding magnitude exceeds an explicitly known whole-block duration.
    PaddingExceedsDuration,
}
/// Timing failure located in source bytes, not a retained-buffer index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimingError {
    /// Invalid element's header offset; missing fields use their enclosing
    /// TrackEntry/Audio header. OpusHead syntax uses the exact offending payload
    /// byte, or first missing byte. These are absolute offsets from source start.
    pub offset: u64,
    /// Encoded element ID, even when an absent field uses its enclosing offset.
    pub element_id: u32,
    /// Typed cause, including explicit unsupported Opus syntax variants.
    pub kind: TimingErrorKind,
}
impl fmt::Display for TimingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "timing element {:#x} at byte {}: {:?}",
            self.element_id, self.offset, self.kind
        )
    }
}
impl std::error::Error for TimingError {}

#[derive(Clone, Copy, Default)]
pub(super) struct Offsets {
    pub entry: u64,
    pub audio: u64,
    pub private: Option<Header>,
    pub delay: Option<Header>,
    pub channels: Option<Header>,
    pub frequency: Option<Header>,
    pub track_type: Option<Header>,
}
pub(super) fn failure(offset: u64, element_id: u32, kind: TimingErrorKind) -> DemuxError {
    DemuxError::Timing(TimingError {
        offset,
        element_id,
        kind,
    })
}
pub(super) fn field(id: u32) -> bool {
    matches!(
        id,
        0x63a2
            | 0x86
            | 0x2ad7b1
            | 0x4489
            | 0x23e383
            | 0x23314f
            | 0x537f
            | 0x56aa
            | 0x56bb
            | 0xb5
            | 0x78b5
            | 0x9f
            | 0x6264
            | 0xe7
            | 0x9b
            | 0x75a2
            | 0xfb
    )
}
pub(super) fn validate(track: &Track, offsets: Offsets) -> Result<Option<OpusTiming>, DemuxError> {
    if track.codec_id != "A_OPUS" {
        return Ok(None);
    }
    if track.track_type != 2 {
        return Err(failure(
            offsets.track_type.map_or(offsets.entry, |h| h.offset),
            0x83,
            TimingErrorKind::OpusTrackType,
        ));
    }
    let private = offsets
        .private
        .ok_or_else(|| failure(offsets.entry, 0x63a2, TimingErrorKind::MissingOpusHead))?;
    let head = opus::parse_opus_head(&track.codec_private).map_err(|error| {
        // Framing checked the finite payload end; the parser's error offset is
        // at most its input length, so this absolute offset cannot overflow.
        failure(
            private.data_offset + error.offset as u64,
            0x63a2,
            TimingErrorKind::OpusHead(error.kind),
        )
    })?;
    let audio = track
        .audio
        .ok_or_else(|| failure(offsets.entry, 0xe1, TimingErrorKind::MissingAudio))?;
    if audio.channels != u64::from(head.channels()) {
        return Err(failure(
            offsets.channels.map_or(offsets.audio, |h| h.offset),
            0x9f,
            TimingErrorKind::ChannelsMismatch,
        ));
    }
    if audio.sampling_frequency != f64::from(head.input_sample_rate()) {
        return Err(failure(
            offsets.frequency.map_or(offsets.audio, |h| h.offset),
            0xb5,
            TimingErrorKind::InputRateMismatch,
        ));
    }
    let difference = head
        .codec_delay_difference(track.codec_delay_ns)
        .ok_or_else(|| {
            failure(
                offsets.delay.map_or(offsets.entry, |h| h.offset),
                0x56aa,
                TimingErrorKind::CodecDelayMismatch,
            )
        })?;
    Ok(Some(OpusTiming {
        pre_skip: head.pre_skip_samples(),
        precision: if difference == 0 {
            DelayPrecision::Exact
        } else {
            DelayPrecision::Quantized { difference }
        },
    }))
}
