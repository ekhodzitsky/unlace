//! Bounded WebM writing from caller-owned encoded blocks.
//!
//! [`LiveWriter`] needs only `Write`; [`IndexedWriter`] adds explicit seekable
//! finalization. Neither encodes media. Successful flush is not storage durability.
//! Dropping or extracting a sink never finalizes it. See the project writing
//! contract for supported metadata, ordering and failure/publication boundaries.
//!
//! ```
//! use unlace::{demux::AudioParameters, write::*};
//! let private = b"OpusHead\x01\x01\0\0\x80\xbb\0\0\0\0\0";
//! let track = TrackConfig {
//!     number: 1, uid: 1, codec_id: "A_OPUS", codec_private: private,
//!     media: MediaParameters::Audio(AudioParameters {
//!         sampling_frequency: 48000.0, output_sampling_frequency: 48000.0,
//!         channels: 1, bit_depth: None,
//!     }),
//!     codec_delay_ns: 0, seek_preroll_ns: 80_000_000,
//!     default_duration_ns: Some(20_000_000), lacing_allowed: true, enabled: true,
//! };
//! let mut writer = LiveWriter::new(Vec::new(), &[track], WriteConfig::default())?;
//! writer.write_block(EncodedBlock::new(1, 0, &[0xf8, 0xff, 0xfe]))?;
//! let receipt = writer.finish()?;
//! assert_eq!(receipt.completion, OutputCompletion::Live);
//! let output = writer.into_inner();
//! assert_eq!(receipt.end_offset, output.len() as u64);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
mod ebml;
use crate::{
    block::{BlockLimits, Lacing},
    demux::{AudioParameters, Track},
};
use ebml::{Bytes, MAX_SIZE, UNKNOWN};
use std::{
    fmt,
    io::{self, Seek, SeekFrom, Write},
};
const SEGMENT: u32 = 0x18538067;
const CLUSTER: u32 = 0x1f43b675;
const INFO: u32 = 0x1549a966;
const TRACKS: u32 = 0x1654ae6b;
const CUES: u32 = 0x1c53bb6b;
const SEEK_HEAD: u32 = 0x114d9b74;
const RESERVATION: usize = 128;

/// Pixel dimensions for the supported VP8/VP9 output tracks.
#[derive(Clone, Copy, Debug)]
pub struct VideoParameters {
    /// Nonzero stored pixel width.
    pub pixel_width: u64,
    /// Nonzero stored pixel height.
    pub pixel_height: u64,
}
/// Track media kind; codec configuration remains borrowed original bytes.
#[derive(Clone, Copy, Debug)]
pub enum MediaParameters {
    /// Opus or Vorbis audio.
    Audio(AudioParameters),
    /// VP8 or VP9 video.
    Video(VideoParameters),
}
/// Initial track metadata. Construction borrows it only until the header is written.
#[derive(Clone, Copy, Debug)]
pub struct TrackConfig<'a> {
    /// Nonzero TrackNumber, at most 56 data bits, shared with blocks.
    pub number: u64,
    /// Nonzero unique TrackUID.
    pub uid: u64,
    /// A_OPUS, A_VORBIS, V_VP8 or V_VP9.
    pub codec_id: &'a str,
    /// Original codec initialization, never rewritten.
    pub codec_private: &'a [u8],
    /// Media-specific container parameters.
    pub media: MediaParameters,
    /// Raw CodecDelay nanoseconds. The writer adds it to presentation time once.
    pub codec_delay_ns: u64,
    /// Raw required decoder preroll nanoseconds, without invented defaults.
    pub seek_preroll_ns: u64,
    /// Per-frame DefaultDuration in nanoseconds, when known.
    pub default_duration_ns: Option<u64>,
    /// Whether this track permits laced blocks.
    pub lacing_allowed: bool,
    /// Preserve the enabled flag, including disabled tracks.
    pub enabled: bool,
}
impl<'a> TrackConfig<'a> {
    /// Borrow supported audio metadata from a demuxed track. Unsupported additions
    /// and nonaudio tracks fail explicitly; video dimensions are not retained by
    /// the audio reader and must be provided separately for video output.
    pub fn from_audio(track: &'a Track) -> Result<Self, WriteError> {
        if track.track_type != 2 || track.max_block_addition_id != 0 {
            return Err(WriteError::Unsupported("audio track or additions"));
        }
        Ok(Self {
            number: track.number,
            uid: track.uid,
            codec_id: &track.codec_id,
            codec_private: &track.codec_private,
            media: MediaParameters::Audio(track.audio.ok_or(WriteError::Invalid("missing Audio"))?),
            codec_delay_ns: track.codec_delay_ns,
            seek_preroll_ns: track.seek_preroll_ns,
            default_duration_ns: track.default_duration_ns,
            lacing_allowed: track.lacing_allowed,
            enabled: track.enabled,
        })
    }
}
/// Original frames and the requested on-disk lacing representation.
#[derive(Clone, Copy, Debug)]
pub enum Frames<'a> {
    /// One frame without lacing. Empty encoded frames are retained.
    Single(&'a [u8]),
    /// Two through 256 frames, with Fixed, Xiph or Ebml lacing.
    Laced {
        /// Original frame slices in source order.
        frames: &'a [&'a [u8]],
        /// Requested length encoding; None is invalid here.
        lacing: Lacing,
    },
}
impl<'a> Frames<'a> {
    fn count(self) -> usize {
        match self {
            Self::Single(_) => 1,
            Self::Laced { frames, .. } => frames.len(),
        }
    }
    fn get(self, i: usize) -> Option<&'a [u8]> {
        match self {
            Self::Single(b) => (i == 0).then_some(b),
            Self::Laced { frames, .. } => frames.get(i).copied(),
        }
    }
    fn lacing(self) -> Lacing {
        match self {
            Self::Single(_) => Lacing::None,
            Self::Laced { lacing, .. } => lacing,
        }
    }
}
/// One complete encoded block. Padding and duration apply to the whole block.
#[derive(Clone, Copy, Debug)]
pub struct EncodedBlock<'a> {
    /// Exact configured TrackNumber.
    pub track_number: u64,
    /// First frame presentation time, in signed nanoseconds after CodecDelay.
    /// It must be exactly representable after adding that delay at the chosen scale.
    pub timestamp_ns: i128,
    /// Caller-owned encoded bytes, borrowed only for this write call.
    pub frames: Frames<'a>,
    /// Optional whole-block duration in nanoseconds, exactly divisible by the scale.
    pub duration_ns: Option<u64>,
    /// Signed whole-block DiscardPadding, never applied to the encoded bytes.
    pub discard_padding_ns: i64,
    /// Whether this block is a random access point, declared by the caller.
    pub keyframe: bool,
    /// SimpleBlock discardable flag. Unavailable on BlockGroup.
    pub discardable: bool,
    /// Invisible flag.
    pub invisible: bool,
    /// Actual ReferenceBlock values in signed Track Ticks (TrackTimestampScale=1).
    /// A non-key BlockGroup with no supplied references writes the standard
    /// unknown-reference value 0. References are never inferred from codec bytes.
    pub references: &'a [i64],
}
impl<'a> EncodedBlock<'a> {
    /// A single caller-declared random-access frame, with no trimming or duration.
    pub fn new(track_number: u64, timestamp_ns: i128, data: &'a [u8]) -> Self {
        Self {
            track_number,
            timestamp_ns,
            frames: Frames::Single(data),
            duration_ns: None,
            discard_padding_ns: 0,
            keyframe: true,
            discardable: false,
            invisible: false,
            references: &[],
        }
    }
}
/// Explicit retained-memory and per-operation work bounds.
#[derive(Clone, Copy, Debug)]
pub struct WriteLimits {
    /// Block, frame, frame-count and lace-header limits.
    pub block: BlockLimits,
    /// Maximum configured track count.
    pub max_tracks: usize,
    /// Maximum serialized initial EBML/Info/Tracks bytes, including headers.
    pub max_metadata_size: usize,
    /// Maximum retained cue entries. Reaching it refuses the next indexed block.
    pub max_cues: usize,
    /// Maximum serialized Cues bytes, checked before accepting a new cue.
    pub max_index_size: u64,
    /// Write/seek/flush calls per public operation, including Interrupted retries.
    pub max_io_calls: u64,
}
impl Default for WriteLimits {
    fn default() -> Self {
        Self {
            block: BlockLimits::default(),
            max_tracks: 64,
            max_metadata_size: 4 * 1024 * 1024,
            max_cues: 65_536,
            max_index_size: 8 * 1024 * 1024,
            max_io_calls: 1_000_000,
        }
    }
}
/// Initial container settings. No duration or codec information is invented.
#[derive(Clone, Copy, Debug)]
pub struct WriteConfig<'a> {
    /// Nanoseconds per tick, strictly positive. Nonrepresentable clocks fail.
    pub timestamp_scale_ns: u64,
    /// MuxingApp and WritingApp text; nonempty and without NUL.
    pub writing_app: &'a str,
    /// Minimum raw time separation between cues for each enabled track. Only
    /// caller-declared keyframes are indexed. Zero indexes every eligible block.
    pub cue_interval_ns: u64,
    /// Resource and work limits.
    pub limits: WriteLimits,
}
impl Default for WriteConfig<'_> {
    fn default() -> Self {
        Self {
            timestamp_scale_ns: 1_000_000,
            writing_app: "unlace",
            cue_interval_ns: 1_000_000_000,
            limits: WriteLimits::default(),
        }
    }
}
/// Observable writer state. Preflight validation failures preserve Writing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteState {
    /// More blocks may be supplied.
    Writing,
    /// Explicit finish and flush succeeded; no more blocks are accepted.
    Finished,
    /// An I/O/partial-output failure is terminal.
    Failed,
}
/// Output stage attached to an I/O failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteStage {
    /// Initial header/metadata.
    Header,
    /// Cluster and encoded block.
    Block,
    /// Final cue entries.
    Cues,
    /// Reserved front SeekHead.
    SeekHead,
    /// Finite Segment length patch and final cursor restoration.
    Commit,
    /// Checked sink flush.
    Flush,
}
/// A checked output failure. Partial sink changes are never rolled back.
#[derive(Debug)]
pub enum WriteError {
    /// CodecPrivate OpusHead syntax, including distinct unsupported version or
    /// mapping kinds and the exact byte offset within the supplied header.
    OpusHead(crate::opus::OpusError),
    /// Invalid configuration or block metadata, before block output.
    Invalid(&'static str),
    /// A feature outside the explicit WebM output scope.
    Unsupported(&'static str),
    /// Memory, encoded size, index or operation work refusal.
    Limit(&'static str),
    /// Sink error, including WouldBlock; offset is the observed cursor, not a
    /// promise that a failed provider made no external changes.
    Io {
        /// Stage which failed.
        stage: WriteStage,
        /// Last confirmed output cursor.
        offset: u64,
        /// Original sink error.
        error: io::Error,
    },
    /// Seek returned a position different from the requested one.
    Position {
        /// Requested position.
        expected: u64,
        /// Provider-reported position.
        actual: u64,
    },
    /// Sink was not empty and positioned at zero for indexed construction.
    NonemptySink,
    /// The writer already finished or failed.
    State(WriteState),
}
impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpusHead(error) => write!(f, "writer initialization: {error}"),
            Self::Invalid(s) => write!(f, "invalid writer input: {s}"),
            Self::Unsupported(s) => write!(f, "unsupported writer feature: {s}"),
            Self::Limit(s) => write!(f, "writer resource limit: {s}"),
            Self::Io {
                stage,
                offset,
                error,
            } => write!(f, "writer {stage:?} at byte {offset}: {error}"),
            Self::Position { expected, actual } => {
                write!(f, "writer seek returned {actual}, expected {expected}")
            }
            Self::NonemptySink => f.write_str("indexed output must be an empty sink at zero"),
            Self::State(s) => write!(f, "writer is {s:?}"),
        }
    }
}
impl std::error::Error for WriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            Self::OpusHead(error) => Some(error),
            _ => None,
        }
    }
}
/// Receipt returned only after explicit finish and successful flush.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriteReceipt {
    /// Live unknown-size Segment or finalized finite/indexed output.
    pub completion: OutputCompletion,
    /// Complete encoded frame count, including laces and disabled tracks.
    pub packets: u64,
    /// Complete block/Cluster count.
    pub blocks: u64,
    /// Absolute end position, exclusive. Not a durability guarantee.
    pub end_offset: u64,
    /// Written cue entries, zero for live output.
    pub cues: usize,
}
/// Distinct stream/file guarantees; neither proves codec decodability or durability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputCompletion {
    /// Unknown-size Segment; a receiver cannot prove the intended live ending.
    Live,
    /// Finite Segment plus Cues and front SeekHead; flush succeeded.
    Indexed,
}

struct Output<W> {
    sink: W,
    position: u64,
    state: WriteState,
    calls: u64,
    maximum: u64,
    stage: WriteStage,
}
impl<W: Write> Output<W> {
    fn new(sink: W, maximum: u64) -> Self {
        Self {
            sink,
            position: 0,
            state: WriteState::Writing,
            calls: 0,
            maximum,
            stage: WriteStage::Header,
        }
    }
    fn begin(&mut self) -> Result<(), WriteError> {
        if self.state != WriteState::Writing {
            return Err(WriteError::State(self.state));
        }
        self.calls = 0;
        Ok(())
    }
    fn call(&mut self) -> Result<(), WriteError> {
        if self.calls >= self.maximum {
            self.state = WriteState::Failed;
            return Err(WriteError::Limit("I/O calls"));
        }
        self.calls += 1;
        Ok(())
    }
    fn fail(&mut self, error: io::Error) -> WriteError {
        self.state = WriteState::Failed;
        WriteError::Io {
            stage: self.stage,
            offset: self.position,
            error,
        }
    }
    fn write(&mut self, mut data: &[u8]) -> Result<(), WriteError> {
        while !data.is_empty() {
            self.call()?;
            match self.sink.write(data) {
                Ok(0) => return Err(self.fail(io::ErrorKind::WriteZero.into())),
                Ok(n) => {
                    if n > data.len() {
                        return Err(self.fail(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "invalid write count",
                        )));
                    }
                    self.position = self.position.checked_add(n as u64).ok_or_else(|| {
                        self.fail(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "output offset overflow",
                        ))
                    })?;
                    data = &data[n..];
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => (),
                Err(e) => return Err(self.fail(e)),
            }
        }
        Ok(())
    }
    fn flush(&mut self) -> Result<(), WriteError> {
        self.stage = WriteStage::Flush;
        loop {
            self.call()?;
            match self.sink.flush() {
                Ok(()) => return Ok(()),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => (),
                Err(e) => return Err(self.fail(e)),
            }
        }
    }
}
impl<W: Write + Seek> Output<W> {
    fn seek_where(&mut self, position: SeekFrom) -> Result<u64, WriteError> {
        loop {
            self.call()?;
            match self.sink.seek(position) {
                Ok(actual) => {
                    self.position = actual;
                    return Ok(actual);
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => (),
                Err(e) => return Err(self.fail(e)),
            }
        }
    }
    fn seek(&mut self, position: u64) -> Result<(), WriteError> {
        let actual = self.seek_where(SeekFrom::Start(position))?;
        if actual != position {
            self.state = WriteState::Failed;
            return Err(WriteError::Position {
                expected: position,
                actual,
            });
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct TrackState {
    number: u64,
    delay: u64,
    default_duration: Option<u64>,
    lacing: bool,
    enabled: bool,
    last_frame: Option<i128>,
    last_cue: Option<u64>,
}
#[derive(Clone, Copy)]
struct Cue {
    time: u64,
    track: u64,
    cluster: u64,
    relative: u64,
}
struct Core<W> {
    out: Output<W>,
    tracks: Vec<TrackState>,
    prefix: Bytes,
    suffix: Bytes,
    envelope: Bytes,
    cues: Vec<Cue>,
    cue_bytes: u64,
    scale: u64,
    interval: u64,
    limits: WriteLimits,
    indexed: bool,
    segment_size_at: u64,
    body: u64,
    info: u64,
    track_offset: u64,
    last_raw: Option<i128>,
    packets: u64,
    blocks: u64,
}

fn validate_track(t: &TrackConfig<'_>) -> Result<(), WriteError> {
    if t.number == 0 || t.number >= (1 << 56) || t.uid == 0 || t.default_duration_ns == Some(0) {
        return Err(WriteError::Invalid("track number, UID or duration"));
    }
    match t.media {
        MediaParameters::Audio(a) => {
            if !a.sampling_frequency.is_finite()
                || a.sampling_frequency <= 0.0
                || !a.output_sampling_frequency.is_finite()
                || a.output_sampling_frequency <= 0.0
                || a.channels == 0
                || a.bit_depth == Some(0)
            {
                return Err(WriteError::Invalid("Audio parameters"));
            }
            match t.codec_id {
                "A_OPUS" => {
                    let h = crate::opus::parse_opus_head(t.codec_private)
                        .map_err(WriteError::OpusHead)?;
                    let difference = i128::from(t.codec_delay_ns) * 48_000
                        - i128::from(h.pre_skip_ns_numerator());
                    if u64::from(h.channels()) != a.channels
                        || f64::from(h.input_sample_rate()) != a.sampling_frequency
                        || !(-48_000..48_000).contains(&difference)
                    {
                        return Err(WriteError::Invalid("Opus channels, rate or delay"));
                    }
                }
                "A_VORBIS" => validate_vorbis(t.codec_private)?,
                _ => return Err(WriteError::Unsupported("audio codec")),
            }
        }
        MediaParameters::Video(v) => {
            if v.pixel_width == 0 || v.pixel_height == 0 {
                return Err(WriteError::Invalid("Video dimensions"));
            }
            if !matches!(t.codec_id, "V_VP8" | "V_VP9") {
                return Err(WriteError::Unsupported("video codec"));
            }
            if t.codec_id == "V_VP8" && !t.codec_private.is_empty() {
                return Err(WriteError::Unsupported("VP8 CodecPrivate"));
            }
        }
    }
    Ok(())
}
fn validate_vorbis(bytes: &[u8]) -> Result<(), WriteError> {
    if bytes.first() != Some(&2) {
        return Err(WriteError::Invalid("Vorbis header count"));
    }
    let mut pos = 1;
    let mut lengths = [0usize; 2];
    for length in &mut lengths {
        loop {
            let b = *bytes
                .get(pos)
                .ok_or(WriteError::Invalid("Vorbis header lengths"))?;
            pos += 1;
            *length = length
                .checked_add(usize::from(b))
                .ok_or(WriteError::Invalid("Vorbis header overflow"))?;
            if b != 255 {
                break;
            }
        }
    }
    for (i, kind) in [1, 3, 5].into_iter().enumerate() {
        let len = if i < 2 {
            lengths[i]
        } else {
            bytes.len().saturating_sub(pos)
        };
        let end = pos
            .checked_add(len)
            .ok_or(WriteError::Invalid("Vorbis header overflow"))?;
        let header = bytes
            .get(pos..end)
            .ok_or(WriteError::Invalid("Vorbis header bounds"))?;
        if header.first() != Some(&kind) || header.get(1..7) != Some(b"vorbis") {
            return Err(WriteError::Invalid("Vorbis identification headers"));
        }
        pos = end;
    }
    Ok(())
}
fn write_track(b: &mut Bytes, t: &TrackConfig<'_>) -> Result<(), WriteError> {
    b.master(0xae, |b| {
        b.uint(0xd7, t.number)?;
        b.uint(0x73c5, t.uid)?;
        b.uint(0xb9, u64::from(t.enabled))?;
        b.uint(0x9c, u64::from(t.lacing_allowed))?;
        b.uint(
            0x83,
            match t.media {
                MediaParameters::Audio(_) => 2,
                MediaParameters::Video(_) => 1,
            },
        )?;
        b.leaf(0x86, t.codec_id.as_bytes())?;
        if !t.codec_private.is_empty() {
            b.leaf(0x63a2, t.codec_private)?;
        }
        b.uint(0x56aa, t.codec_delay_ns)?;
        b.uint(0x56bb, t.seek_preroll_ns)?;
        if let Some(d) = t.default_duration_ns {
            b.uint(0x23e383, d)?;
        }
        match t.media {
            MediaParameters::Audio(a) => b.master(0xe1, |b| {
                b.leaf(0xb5, &a.sampling_frequency.to_be_bytes())?;
                b.leaf(0x78b5, &a.output_sampling_frequency.to_be_bytes())?;
                b.uint(0x9f, a.channels)?;
                if let Some(depth) = a.bit_depth {
                    b.uint(0x6264, depth)?;
                }
                Ok(())
            }),
            MediaParameters::Video(v) => b.master(0xe0, |b| {
                b.uint(0xb0, v.pixel_width)?;
                b.uint(0xba, v.pixel_height)
            }),
        }
    })
}
fn cue(b: &mut Bytes, c: Cue) -> Result<(), WriteError> {
    b.master(0xbb, |b| {
        b.leaf(0xb3, &c.time.to_be_bytes())?;
        b.master(0xb7, |b| {
            b.leaf(0xf7, &c.track.to_be_bytes())?;
            b.leaf(0xf1, &c.cluster.to_be_bytes())?;
            b.leaf(0xf0, &c.relative.to_be_bytes())
        })
    })
}
const CUE_BYTES: u64 = 58;
impl<W: Write> Core<W> {
    fn new(
        mut out: Output<W>,
        tracks: &[TrackConfig<'_>],
        config: WriteConfig<'_>,
        indexed: bool,
    ) -> Result<Self, WriteError> {
        let limits = config.limits;
        if config.timestamp_scale_ns == 0
            || config.writing_app.is_empty()
            || tracks.is_empty()
            || !(1..=256).contains(&limits.block.max_frames)
            || limits.max_io_calls == 0
        {
            return Err(WriteError::Invalid("writer configuration"));
        }
        if tracks.len() > limits.max_tracks {
            return Err(WriteError::Limit("tracks"));
        }
        // Bound all variable metadata before codec/header scanning or allocation.
        let mut variable = config
            .writing_app
            .len()
            .checked_mul(2)
            .ok_or(WriteError::Limit("metadata size"))?;
        for t in tracks {
            variable = variable
                .checked_add(t.codec_private.len())
                .and_then(|n| n.checked_add(t.codec_id.len()))
                .ok_or(WriteError::Limit("metadata size"))?;
        }
        if variable > limits.max_metadata_size {
            return Err(WriteError::Limit("metadata size"));
        }
        if config.writing_app.contains('\0') {
            return Err(WriteError::Invalid("writing application text"));
        }
        let prefix_limit = limits.max_metadata_size.max(
            limits
                .block
                .max_lace_header_size
                .checked_add(16)
                .ok_or(WriteError::Invalid("lace limits"))?,
        );
        for (i, t) in tracks.iter().enumerate() {
            validate_track(t)?;
            if tracks[..i]
                .iter()
                .any(|old| old.number == t.number || old.uid == t.uid)
            {
                return Err(WriteError::Invalid("duplicate track identity"));
            }
        }
        let mut prefix = Bytes::new(limits.max_metadata_size);
        prefix.master(0x1a45dfa3, |b| {
            b.uint(0x4286, 1)?;
            b.uint(0x42f7, 1)?;
            b.uint(0x42f2, 4)?;
            b.uint(0x42f3, 8)?;
            b.leaf(0x4282, b"webm")?;
            b.uint(0x4287, 4)?;
            b.uint(0x4285, 2)
        })?;
        prefix.id(SEGMENT)?;
        let segment_size_at = prefix.data.len() as u64;
        prefix.raw(&UNKNOWN)?;
        let body = prefix.data.len() as u64;
        if indexed {
            prefix.leaf(0xec, &[0; RESERVATION - 2])?;
        }
        let info = prefix.data.len() as u64 - body;
        prefix.master(INFO, |b| {
            b.uint(0x2ad7b1, config.timestamp_scale_ns)?;
            b.leaf(0x4d80, config.writing_app.as_bytes())?;
            b.leaf(0x5741, config.writing_app.as_bytes())
        })?;
        let track_offset = prefix.data.len() as u64 - body;
        prefix.master(TRACKS, |b| {
            for t in tracks {
                write_track(b, t)?;
            }
            Ok(())
        })?;
        let mut states = Vec::new();
        states
            .try_reserve_exact(tracks.len())
            .map_err(|_| WriteError::Limit("track allocation"))?;
        for t in tracks {
            states.push(TrackState {
                number: t.number,
                delay: t.codec_delay_ns,
                default_duration: t.default_duration_ns,
                lacing: t.lacing_allowed,
                enabled: t.enabled,
                last_frame: None,
                last_cue: None,
            });
        }
        out.write(&prefix.data)?;
        prefix.clear();
        prefix.limit = prefix_limit;
        Ok(Self {
            out,
            tracks: states,
            prefix,
            suffix: Bytes::new(3072),
            envelope: Bytes::new(128),
            cues: Vec::new(),
            cue_bytes: 0,
            scale: config.timestamp_scale_ns,
            interval: config.cue_interval_ns,
            limits,
            indexed,
            segment_size_at,
            body,
            info,
            track_offset,
            last_raw: None,
            packets: 0,
            blocks: 0,
        })
    }
    fn write_block(&mut self, b: EncodedBlock<'_>) -> Result<(), WriteError> {
        self.out.begin()?;
        let index = self
            .tracks
            .iter()
            .position(|t| t.number == b.track_number)
            .ok_or(WriteError::Invalid("unknown TrackNumber"))?;
        let track = *self
            .tracks
            .get(index)
            .ok_or(WriteError::Invalid("track bounds"))?;
        let count = b.frames.count();
        let mode = b.frames.lacing();
        if count == 0 || count > 256 {
            return Err(WriteError::Invalid("frame count"));
        }
        if count > self.limits.block.max_frames {
            return Err(WriteError::Limit("frame count"));
        }
        if matches!(b.frames, Frames::Laced { .. }) && (count < 2 || mode == Lacing::None) {
            return Err(WriteError::Invalid("laced frame representation"));
        }
        if count == 1 && mode != Lacing::None
            || count > 1 && (mode == Lacing::None || !track.lacing)
        {
            return Err(WriteError::Invalid("lacing mode or track permission"));
        }
        if b.references.len() > 256 {
            return Err(WriteError::Limit("references"));
        }
        if b.keyframe && !b.references.is_empty()
            || b.references.len() > 1 && b.references.contains(&0)
        {
            return Err(WriteError::Invalid("block references"));
        }
        if b.duration_ns == Some(0) {
            return Err(WriteError::Invalid("zero block duration"));
        }
        let raw = b
            .timestamp_ns
            .checked_add(i128::from(track.delay))
            .ok_or(WriteError::Invalid("timestamp overflow"))?;
        if raw % i128::from(self.scale) != 0 {
            return Err(WriteError::Invalid(
                "timestamp is not exactly representable",
            ));
        }
        if self.last_raw.is_some_and(|last| raw < last)
            || track.last_frame.is_some_and(|last| b.timestamp_ns < last)
        {
            return Err(WriteError::Invalid("nonmonotonic timestamp"));
        }
        let ticks = raw / i128::from(self.scale);
        let cluster = u64::try_from(ticks.max(0))
            .map_err(|_| WriteError::Invalid("Cluster timestamp range"))?;
        let relative = i16::try_from(ticks.min(0))
            .map_err(|_| WriteError::Invalid("negative relative timestamp range"))?;
        let duration = match b.duration_ns {
            Some(d) if d % self.scale != 0 => {
                return Err(WriteError::Invalid("duration is not exactly representable"));
            }
            Some(d) => Some(d / self.scale),
            None => None,
        };
        let known_duration = b.duration_ns.map(i128::from).or_else(|| {
            track
                .default_duration
                .map(|d| i128::from(d) * count as i128)
        });
        if known_duration.is_some_and(|d| i128::from(b.discard_padding_ns).abs() > d) {
            return Err(WriteError::Invalid("padding exceeds block duration"));
        }
        let last_frame = if count == 1 {
            Some(b.timestamp_ns)
        } else {
            match track.default_duration {
                Some(d) => Some(
                    b.timestamp_ns
                        .checked_add(i128::from(d) * (count - 1) as i128)
                        .ok_or(WriteError::Invalid("frame timestamp overflow"))?,
                ),
                None => None,
            }
        };
        let packets = self
            .packets
            .checked_add(count as u64)
            .ok_or(WriteError::Limit("packet counter"))?;
        let blocks = self
            .blocks
            .checked_add(1)
            .ok_or(WriteError::Limit("block counter"))?;
        let group = duration.is_some() || b.discard_padding_ns != 0 || !b.references.is_empty();
        if group && b.discardable {
            return Err(WriteError::Unsupported("BlockGroup discardable flag"));
        }
        self.prefix.clear();
        self.suffix.clear();
        self.envelope.clear();
        let (v, w) = ebml::vint(b.track_number)?;
        self.prefix.raw(&v[8 - w..])?;
        self.prefix.raw(&relative.to_be_bytes())?;
        let flags = (if b.invisible { 8 } else { 0 })
            | (if !group && b.keyframe { 0x80 } else { 0 })
            | (if !group && b.discardable { 1 } else { 0 })
            | match mode {
                Lacing::None => 0,
                Lacing::Xiph => 2,
                Lacing::Fixed => 4,
                Lacing::Ebml => 6,
            };
        self.prefix.raw(&[flags])?;
        let lace_start = self.prefix.data.len();
        if count > 1 {
            self.prefix.raw(&[(count - 1) as u8])?;
            let first = b
                .frames
                .get(0)
                .ok_or(WriteError::Invalid("frame bounds"))?
                .len();
            for i in 0..count - 1 {
                let len = b
                    .frames
                    .get(i)
                    .ok_or(WriteError::Invalid("frame bounds"))?
                    .len();
                match mode {
                    Lacing::Fixed => {
                        if len != first || b.frames.get(count - 1).is_none_or(|f| f.len() != first)
                        {
                            return Err(WriteError::Invalid("unequal fixed lace"));
                        }
                    }
                    Lacing::Xiph => {
                        let mut rest = len;
                        while rest >= 255 {
                            if self.prefix.data.len() - lace_start
                                >= self.limits.block.max_lace_header_size
                            {
                                return Err(WriteError::Limit("lace header"));
                            }
                            self.prefix.raw(&[255])?;
                            rest -= 255;
                        }
                        self.prefix.raw(&[rest as u8])?;
                    }
                    Lacing::Ebml => {
                        let (bytes, width) = if i == 0 {
                            ebml::vint(len as u64)?
                        } else {
                            let previous = b
                                .frames
                                .get(i - 1)
                                .ok_or(WriteError::Invalid("frame bounds"))?
                                .len();
                            let delta = len as i128 - previous as i128;
                            let width = (1..=8)
                                .find(|w| {
                                    let bias = (1i128 << (7 * w - 1)) - 1;
                                    delta >= -bias && delta <= bias + 1
                                })
                                .ok_or(WriteError::Invalid("lace difference"))?;
                            let bias = (1i128 << (7 * width - 1)) - 1;
                            (
                                ((delta + bias) as u64 | (1u64 << (7 * width))).to_be_bytes(),
                                width,
                            )
                        };
                        self.prefix.raw(&bytes[8 - width..])?;
                    }
                    Lacing::None => return Err(WriteError::Invalid("lacing")),
                }
            }
        }
        if self.prefix.data.len() - lace_start > self.limits.block.max_lace_header_size {
            return Err(WriteError::Limit("lace header"));
        }
        let mut block_size = self.prefix.data.len() as u64;
        for i in 0..count {
            let frame = b.frames.get(i).ok_or(WriteError::Invalid("frame bounds"))?;
            if frame.len() > self.limits.block.max_frame_size {
                return Err(WriteError::Limit("frame size"));
            }
            block_size = block_size
                .checked_add(frame.len() as u64)
                .ok_or(WriteError::Limit("block size"))?;
        }
        if block_size > self.limits.block.max_block_size as u64 {
            return Err(WriteError::Limit("block size"));
        }
        if let Some(d) = duration {
            self.suffix.uint(0x9b, d)?;
        }
        if b.discard_padding_ns != 0 {
            self.suffix.signed(0x75a2, b.discard_padding_ns)?;
        }
        for &r in b.references {
            self.suffix.signed(0xfb, r)?;
        }
        if group && !b.keyframe && b.references.is_empty() {
            self.suffix.signed(0xfb, 0)?;
        }
        // Assemble only headers; encoded frame bytes go directly to the sink.
        self.envelope.uint(0xe7, cluster)?;
        let cue_relative = self.envelope.data.len() as u64;
        let (_, bw) = ebml::size(block_size, None)?;
        let block_element = 1 + bw as u64 + block_size;
        let group_size = block_element + self.suffix.data.len() as u64;
        let group_header = if group {
            let (_, w) = ebml::size(group_size, None)?;
            1 + w as u64
        } else {
            0
        };
        let cluster_size = cue_relative + group_header + group_size;
        // Prefix Cluster header by shifting the small Timestamp header.
        self.envelope.prepend_header(CLUSTER, cluster_size)?;
        if group {
            self.envelope.header(0xa0, group_size)?;
        }
        self.envelope
            .header(if group { 0xa1 } else { 0xa3 }, block_size)?;
        let end = self
            .out
            .position
            .checked_add(self.envelope.data.len() as u64)
            .and_then(|n| n.checked_add(block_size))
            .and_then(|n| n.checked_add(self.suffix.data.len() as u64))
            .ok_or(WriteError::Limit("output offset"))?;
        if end - self.body > MAX_SIZE {
            return Err(WriteError::Limit("Segment size"));
        }
        let cue_time = u64::try_from(ticks).ok();
        let add_cue = self.indexed
            && track.enabled
            && b.keyframe
            && cue_time.is_some_and(|t| {
                track.last_cue.is_none_or(|old| {
                    (t - old) as u128 * u128::from(self.scale) >= u128::from(self.interval)
                })
            });
        let next_cue_bytes = if add_cue {
            self.cue_bytes
                .checked_add(CUE_BYTES)
                .ok_or(WriteError::Limit("index size"))?
        } else {
            self.cue_bytes
        };
        if add_cue {
            let (_, width) = ebml::size(next_cue_bytes, None)?;
            if self.cues.len() >= self.limits.max_cues
                || next_cue_bytes + 4 + width as u64 > self.limits.max_index_size
            {
                return Err(WriteError::Limit("Cues"));
            }
            self.cues
                .try_reserve(1)
                .map_err(|_| WriteError::Limit("cue allocation"))?;
        }
        let cluster_offset = self.out.position - self.body;
        self.out.stage = WriteStage::Block;
        self.out.write(&self.envelope.data)?;
        self.out.write(&self.prefix.data)?;
        for i in 0..count {
            self.out
                .write(b.frames.get(i).ok_or(WriteError::Invalid("frame bounds"))?)?;
        }
        self.out.write(&self.suffix.data)?;
        let track = self
            .tracks
            .get_mut(index)
            .ok_or(WriteError::Invalid("track bounds"))?;
        if add_cue {
            let time = cue_time.ok_or(WriteError::Invalid("cue time"))?;
            self.cues.push(Cue {
                time,
                track: track.number,
                cluster: cluster_offset,
                relative: cue_relative,
            });
            track.last_cue = Some(time);
            self.cue_bytes = next_cue_bytes;
        }
        track.last_frame = last_frame;
        self.last_raw = Some(raw);
        self.packets = packets;
        self.blocks = blocks;
        Ok(())
    }
    fn receipt(&self, completion: OutputCompletion) -> WriteReceipt {
        WriteReceipt {
            completion,
            packets: self.packets,
            blocks: self.blocks,
            end_offset: self.out.position,
            cues: self.cues.len(),
        }
    }
}

/// Streaming writer over a fresh, exclusive `Write` sink. Segment size stays
/// unknown even after finish; no index/history is retained in live mode.
pub struct LiveWriter<W> {
    core: Core<W>,
}
impl<W: Write> LiveWriter<W> {
    /// Write the initial header/metadata. On failure the sink may contain a
    /// prefix; pass a mutable borrow if the caller needs to retain that sink.
    pub fn new(
        sink: W,
        tracks: &[TrackConfig<'_>],
        config: WriteConfig<'_>,
    ) -> Result<Self, WriteError> {
        Ok(Self {
            core: Core::new(
                Output::new(sink, config.limits.max_io_calls),
                tracks,
                config,
                false,
            )?,
        })
    }
    /// Validate the entire block before emitting any of it. I/O failures are
    /// terminal, including WouldBlock, so retry cannot duplicate a partial block.
    pub fn write_block(&mut self, block: EncodedBlock<'_>) -> Result<(), WriteError> {
        self.core.write_block(block)
    }
    /// Flush explicitly and return a live-only receipt. No on-wire final marker
    /// or intended source completeness can be inferred by a receiving demuxer.
    pub fn finish(&mut self) -> Result<WriteReceipt, WriteError> {
        self.core.out.begin()?;
        if self.core.blocks == 0 {
            return Err(WriteError::Invalid("no blocks"));
        }
        self.core.out.flush()?;
        self.core.out.state = WriteState::Finished;
        Ok(self.core.receipt(OutputCompletion::Live))
    }
    /// Current terminal/writing state.
    pub fn state(&self) -> WriteState {
        self.core.out.state
    }
    /// Last confirmed absolute output cursor, relative to the fresh sink start.
    pub fn position(&self) -> u64 {
        self.core.out.position
    }
    /// Recover the sink without implicit finish, flush or successful receipt.
    pub fn into_inner(self) -> W {
        self.core.out.sink
    }
}
/// Indexed WebM output over an empty, exclusively owned `Write + Seek` sink.
/// Header size remains unknown until explicit checked finalization.
pub struct IndexedWriter<W> {
    core: Core<W>,
}
impl<W: Write + Seek> IndexedWriter<W> {
    /// Require an empty sink at zero, then write the metadata and front index
    /// reservation. A borrowed sink remains available to the caller on failure.
    pub fn new(
        sink: W,
        tracks: &[TrackConfig<'_>],
        config: WriteConfig<'_>,
    ) -> Result<Self, WriteError> {
        // Probe without writing. The provider is required to implement Seek
        // honestly; position equality cannot authenticate externally mutable bytes.
        let mut out = Output::new(sink, config.limits.max_io_calls);
        let at = out.seek_where(SeekFrom::Current(0))?;
        let end = out.seek_where(SeekFrom::End(0))?;
        if at != 0 || end != 0 {
            return Err(WriteError::NonemptySink);
        }
        out.seek(0)?;
        Ok(Self {
            core: Core::new(out, tracks, config, true)?,
        })
    }
    /// Write one validated block/finite Cluster. Cue limits are checked and
    /// reserved before output; no cue or packet is silently dropped.
    pub fn write_block(&mut self, block: EncodedBlock<'_>) -> Result<(), WriteError> {
        self.core.write_block(block)
    }
    /// Append Cues, patch the front SeekHead, flush, patch the finite Segment
    /// size, restore the end cursor and flush again. Only then return a receipt.
    /// Generic failing sinks can retain complete-looking bytes after a commit
    /// or final-flush error; a receipt and staged publication are required when
    /// successful creation must be observable outside the writer.
    pub fn finish(&mut self) -> Result<WriteReceipt, WriteError> {
        self.core.out.begin()?;
        if self.core.cues.is_empty() {
            return Err(WriteError::Invalid(
                "no nonnegative indexed random access block",
            ));
        }
        self.core
            .prefix
            .data
            .try_reserve(128)
            .map_err(|_| WriteError::Limit("finalization allocation"))?;
        let result = self.finish_inner();
        if result.is_err() {
            self.core.out.state = WriteState::Failed;
        }
        result
    }
    fn finish_inner(&mut self) -> Result<WriteReceipt, WriteError> {
        let c = &mut self.core;
        let cues_at = c.out.position - c.body;
        c.prefix.clear();
        c.prefix.header(CUES, c.cue_bytes)?;
        let end = c
            .out
            .position
            .checked_add(c.prefix.data.len() as u64)
            .and_then(|n| n.checked_add(c.cue_bytes))
            .ok_or(WriteError::Limit("final output offset"))?;
        if end - c.body > MAX_SIZE {
            return Err(WriteError::Limit("Segment size"));
        }
        c.out.stage = WriteStage::Cues;
        c.out.write(&c.prefix.data)?;
        for &entry in &c.cues {
            c.prefix.clear();
            cue(&mut c.prefix, entry)?;
            c.out.write(&c.prefix.data)?;
        }
        c.prefix.clear();
        c.prefix.master(SEEK_HEAD, |b| {
            for (id, position) in [(INFO, c.info), (TRACKS, c.track_offset), (CUES, cues_at)] {
                b.master(0x4dbb, |b| {
                    b.leaf(0x53ab, &id.to_be_bytes())?;
                    b.leaf(0x53ac, &position.to_be_bytes())
                })?;
            }
            Ok(())
        })?;
        let void = RESERVATION
            .checked_sub(c.prefix.data.len() + 2)
            .ok_or(WriteError::Limit("SeekHead reservation"))?;
        let zeros = [0; RESERVATION];
        c.prefix.leaf(0xec, &zeros[..void])?;
        c.out.stage = WriteStage::SeekHead;
        c.out.seek(c.body)?;
        c.out.write(&c.prefix.data)?;
        c.out.flush()?;
        let (size, _) = ebml::size(end - c.body, Some(8))?;
        c.out.stage = WriteStage::Commit;
        c.out.seek(c.segment_size_at)?;
        c.out.write(&size)?;
        c.out.seek(end)?;
        c.out.flush()?;
        c.out.state = WriteState::Finished;
        Ok(c.receipt(OutputCompletion::Indexed))
    }
    /// Current terminal/writing state.
    pub fn state(&self) -> WriteState {
        self.core.out.state
    }
    /// Last confirmed output cursor. A failed patch may leave it in the header.
    pub fn position(&self) -> u64 {
        self.core.out.position
    }
    /// Recover output without completing or repairing partial finalization.
    pub fn into_inner(self) -> W {
        self.core.out.sink
    }
}
