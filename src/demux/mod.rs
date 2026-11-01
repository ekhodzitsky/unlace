//! Sequential, bounded WebM/Matroska audio packet demuxing over [`Read`].
//!
//! Packet bytes borrow the demuxer until the next mutable call. Poll through to
//! End to check final structure; receiving packets alone does not prove success.
//! The supported subset and explicit exclusions are described in project docs.
mod schema;

use crate::{
    ElementSize,
    block::{self, BlockKind, BlockLimits, Lacing},
    stream::{self, Header, Kind},
};
use std::{fmt, io::Read, ops::Range};

const EBML: u32 = 0x1a45dfa3;
const SEGMENT: u32 = 0x18538067;
const INFO: u32 = 0x1549a966;
const TRACKS: u32 = 0x1654ae6b;
const CLUSTER: u32 = 0x1f43b675;

/// Which audio track to emit. Other tracks remain visible in metadata.
#[derive(Clone, Copy, Debug)]
pub enum AudioSelection {
    /// First enabled audio TrackEntry in file order.
    First,
    /// Exact nonzero TrackNumber, required to identify an audio track.
    Number(u64),
}

/// Memory and work bounds independent of recording duration.
#[derive(Clone, Copy, Debug)]
pub struct DemuxLimits {
    /// Low-level framing limits, including depth and work per framing poll.
    pub framing: stream::Limits,
    /// Block size, frame and lacing work limits.
    pub block: BlockLimits,
    /// Maximum retained track count.
    pub max_tracks: usize,
    /// Combined declared payload sizes of EBML, Info and Tracks masters.
    /// Includes all their descendants and skipped bytes, counted once.
    pub max_metadata_size: u64,
    /// Maximum framing events processed per demux poll.
    pub max_events_per_poll: usize,
}
impl Default for DemuxLimits {
    fn default() -> Self {
        Self {
            framing: stream::Limits::default(),
            block: BlockLimits::default(),
            max_tracks: 64,
            max_metadata_size: 4 * 1024 * 1024,
            max_events_per_poll: 64,
        }
    }
}

/// Parameters retained for every track. Codec bytes are never decoded.
#[derive(Debug)]
pub struct Track {
    /// Stable TrackNumber used by blocks.
    pub number: u64,
    /// Nonzero TrackUID.
    pub uid: u64,
    /// Matroska TrackType; 2 means audio.
    pub track_type: u64,
    /// Codec ID from the container.
    pub codec_id: String,
    /// Unmodified codec initialization bytes.
    pub codec_private: Vec<u8>,
    /// CodecDelay in nanoseconds. Packet timestamps already subtract it once.
    pub codec_delay_ns: u64,
    /// Required decoder preroll in nanoseconds; no decoding is performed here.
    pub seek_preroll_ns: u64,
    /// Declared maximum BlockAddID (default 0). This declaration alone does not
    /// imply that additions occur. Selected-track BlockAdditions are unsupported.
    pub max_block_addition_id: u64,
    /// Default encoded frame duration in nanoseconds, when supplied.
    pub default_duration_ns: Option<u64>,
    /// Whether lacing is permitted on this track.
    pub lacing_allowed: bool,
    /// Track enabled flag.
    pub enabled: bool,
    /// Audio settings, present for audio tracks only.
    pub audio: Option<AudioParameters>,
}

/// Container audio parameters; these do not promise decoded output properties.
#[derive(Clone, Copy, Debug)]
pub struct AudioParameters {
    /// SamplingFrequency in hertz (default 8000).
    pub sampling_frequency: f64,
    /// OutputSamplingFrequency in hertz, defaulting to SamplingFrequency.
    pub output_sampling_frequency: f64,
    /// Nonzero channel count (default 1).
    pub channels: u64,
    /// Bit depth when declared.
    pub bit_depth: Option<u64>,
}

/// Segment timing, available after Info has been read.
#[derive(Clone, Copy, Debug)]
pub struct SegmentInfo {
    /// Nanoseconds per Segment Tick (default 1,000,000).
    pub timestamp_scale_ns: u64,
    /// Optional advisory Duration in Segment Ticks; not a completeness proof.
    pub duration_ticks: Option<f64>,
}

/// Evidence at final source EOF. Neither case proves producer completeness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion {
    /// A finite Segment boundary and final EOF were both observed.
    FiniteSegment,
    /// EOF ended an unknown-size Segment. Missing intended packets cannot be
    /// detected from this framing alone, even if every observed element parsed.
    UnknownSizeSegment,
}

/// One original encoded packet and its container metadata.
#[derive(Debug)]
pub struct Packet<'a> {
    /// Original encoded bytes, never trimmed or transformed.
    pub data: &'a [u8],
    /// Stable TrackNumber.
    pub track_number: u64,
    /// Presentation timestamp in nanoseconds, with CodecDelay subtracted once.
    /// Later laced frames without DefaultDuration have underdetermined times
    /// and return None; their order and shared block origin remain explicit.
    pub timestamp_ns: Option<i128>,
    /// First frame's timestamp before CodecDelay, in nanoseconds.
    pub block_timestamp_ns: i128,
    /// Zero-based frame index inside the original block.
    pub lace_index: u16,
    /// Number of original frames in the block.
    pub lace_count: u16,
    /// Absolute offset of the original Block/SimpleBlock element.
    pub block_offset: u64,
    /// DefaultDuration for this frame, when supplied; not codec-derived.
    pub default_duration_ns: Option<u64>,
    /// Total BlockDuration in nanoseconds, when supplied, for the whole block.
    pub block_duration_ns: Option<i128>,
    /// Signed DiscardPadding, in nanoseconds. Positive padding belongs to the
    /// last laced frame, negative padding to the first; zero elsewhere.
    pub discard_padding_ns: i64,
    /// SimpleBlock flag or absence of ReferenceBlock in a BlockGroup.
    pub keyframe: bool,
    /// SimpleBlock discardable flag; false for BlockGroup.
    pub discardable: bool,
    /// Invisible flag.
    pub invisible: bool,
}

/// Observable progress. A packet's storage is valid until the next mutable call.
#[derive(Debug)]
pub enum DemuxEvent<'a> {
    /// Tracks are finalized; inspect [`Demuxer::tracks`]. Selection succeeded.
    TracksReady,
    /// Next selected audio packet in container order.
    Packet(Packet<'a>),
    /// Source returned WouldBlock; state is retained for the next poll.
    NeedInput,
    /// Work budget reached; poll again to continue.
    Yield,
    /// Final source EOF after structural and required-metadata checks.
    End(Completion),
}

/// A demux failure. Except configuration errors from construction, errors are
/// terminal and further polling returns Failed.
#[derive(Debug)]
pub enum DemuxError {
    /// Low-level syntax, source, truncation or framing resource error.
    Stream(stream::StreamError),
    /// Invalid block syntax or exceeded block limits.
    Block(block::BlockError),
    /// Invalid supported container structure or scalar value.
    Invalid(&'static str),
    /// A valid or potentially valid feature outside the supported subset.
    Unsupported(&'static str),
    /// Selected audio track is absent, disabled or has a nonaudio type.
    AudioTrackUnavailable,
    /// Track, aggregate metadata or allocation refusal.
    Limit(&'static str),
    /// Invalid configuration before reading.
    Configuration,
    /// Continuation was poisoned by an earlier failure.
    Failed,
}
impl fmt::Display for DemuxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stream(error) => write!(f, "container framing: {error}"),
            Self::Block(error) => write!(f, "container block: {error}"),
            Self::Invalid(reason) => write!(f, "invalid container: {reason}"),
            Self::Unsupported(reason) => write!(f, "unsupported container feature: {reason}"),
            Self::AudioTrackUnavailable => f.write_str("selected audio track is unavailable"),
            Self::Limit(reason) => write!(f, "container resource limit: {reason}"),
            Self::Configuration => f.write_str("invalid demux configuration"),
            Self::Failed => f.write_str("demuxer previously failed"),
        }
    }
}
impl std::error::Error for DemuxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Stream(error) => Some(error),
            Self::Block(error) => Some(error),
            _ => None,
        }
    }
}
impl From<stream::StreamError> for DemuxError {
    fn from(error: stream::StreamError) -> Self {
        Self::Stream(error)
    }
}
impl From<block::BlockError> for DemuxError {
    fn from(error: block::BlockError) -> Self {
        Self::Block(error)
    }
}

#[derive(Clone, Copy, Default)]
struct Context {
    id: u32,
    seen: u128,
}
struct HeaderState {
    doc_type: Option<String>,
    version: u64,
    read_version: u64,
    id_width: u64,
    size_width: u64,
    doc_version: u64,
    doc_read_version: u64,
}
impl Default for HeaderState {
    fn default() -> Self {
        Self {
            doc_type: None,
            version: 1,
            read_version: 1,
            id_width: 4,
            size_width: 8,
            doc_version: 1,
            doc_read_version: 1,
        }
    }
}
#[derive(Default)]
struct TrackState {
    track: Option<Track>,
    output_frequency: Option<f64>,
}
#[derive(Default)]
struct Group {
    additions: bool,
    header: Option<Header>,
    reference: bool,
    duration: Option<u64>,
    padding: i64,
}
struct Packets {
    ranges: [Range<usize>; 256],
    count: usize,
    next: usize,
    track: usize,
    header: Header,
    timestamp: i128,
    duration: Option<i128>,
    padding: i64,
    keyframe: bool,
    discardable: bool,
    invisible: bool,
}

enum Action {
    Tracks,
    Packet,
    NeedInput,
    Yield,
    End(Completion),
}

/// Single-document sequential audio demuxer owning its Read source.
///
/// Memory is bounded by declared metadata/block limits, retained track count and
/// fixed context/frame tables. No Seek, codec, networking or external tools are
/// required. A blocking Read may block; work limits bound calls, not latency.
/// The first release of this layer rejects CRC-32, content encodings, codec
/// state changes, selected-track additions, nonunit TrackTimestampScale and
/// chained documents explicitly. Unselected additions are skipped opaquely under
/// finite boundary and skip-size limits once the group track is resolved.
/// Known ancillary finite masters may be skipped opaquely as documented.
pub struct Demuxer<R> {
    reader: stream::Reader<'static, R>,
    limits: DemuxLimits,
    selection: AudioSelection,
    contexts: [Context; 64],
    depth: usize,
    root_seen: u128,
    header: HeaderState,
    header_done: bool,
    segment_seen: bool,
    segment_done: bool,
    completion: Completion,
    info: SegmentInfo,
    info_done: bool,
    tracks_done: bool,
    metadata_size: u64,
    tracks: Vec<Track>,
    building_track: TrackState,
    selected: Option<usize>,
    cluster_timestamp: Option<u64>,
    group: Option<Group>,
    leaf: Option<Header>,
    metadata: Vec<u8>,
    block_data: Vec<u8>,
    packets: Option<Packets>,
    failed: bool,
}

impl<R: Read> Demuxer<R> {
    /// Construct without consuming input. Selection is finalized at Tracks end.
    pub fn new(
        source: R,
        selection: AudioSelection,
        limits: DemuxLimits,
    ) -> Result<Self, DemuxError> {
        if limits.max_tracks == 0
            || limits.max_events_per_poll == 0
            || !(1..=256).contains(&limits.block.max_frames)
            || matches!(selection, AudioSelection::Number(0))
        {
            return Err(DemuxError::Configuration);
        }
        let schema = stream::Schema::new(schema::ENTRIES)
            .map_err(|_| DemuxError::Invalid("internal schema"))?;
        Ok(Self {
            reader: stream::Reader::new(source, schema, limits.framing)?,
            limits,
            selection,
            contexts: [Context::default(); 64],
            depth: 0,
            root_seen: 0,
            header: HeaderState::default(),
            header_done: false,
            segment_seen: false,
            segment_done: false,
            completion: Completion::UnknownSizeSegment,
            info: SegmentInfo {
                timestamp_scale_ns: 1_000_000,
                duration_ticks: None,
            },
            info_done: false,
            tracks_done: false,
            metadata_size: 0,
            tracks: Vec::new(),
            building_track: TrackState::default(),
            selected: None,
            cluster_timestamp: None,
            group: None,
            leaf: None,
            metadata: Vec::new(),
            block_data: Vec::new(),
            packets: None,
            failed: false,
        })
    }

    /// Finalized track metadata in file order. Partial tracks are not exposed.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Segment timing after Info has ended.
    pub fn info(&self) -> Option<SegmentInfo> {
        self.info_done.then_some(self.info)
    }

    /// Consumed bytes, including any retained framing lookahead.
    pub fn position(&self) -> u64 {
        self.reader.position()
    }

    /// Recover the source, discarding continuation and any buffered lookahead.
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }

    /// Advance bounded work and borrow the next packet, if available. Always
    /// continue through End to detect late truncation, corruption or source errors.
    pub fn poll(&mut self) -> Result<DemuxEvent<'_>, DemuxError> {
        if self.failed {
            return Err(DemuxError::Failed);
        }
        let action = match self.advance() {
            Ok(action) => action,
            Err(error) => {
                self.failed = true;
                return Err(error);
            }
        };
        Ok(match action {
            Action::Tracks => DemuxEvent::TracksReady,
            Action::NeedInput => DemuxEvent::NeedInput,
            Action::Yield => DemuxEvent::Yield,
            Action::End(completion) => DemuxEvent::End(completion),
            Action::Packet => {
                // advance only returns Packet with a nonempty pending table.
                let pending = self
                    .packets
                    .as_mut()
                    .ok_or(DemuxError::Invalid("missing packet state"))?;
                let index = pending.next;
                pending.next += 1;
                let track = &self.tracks[pending.track];
                let timestamp_ns = if index == 0 {
                    Some(pending.timestamp - i128::from(track.codec_delay_ns))
                } else {
                    track.default_duration_ns.map(|duration| {
                        pending.timestamp - i128::from(track.codec_delay_ns)
                            + index as i128 * i128::from(duration)
                    })
                };
                let discard_padding_ns = if (pending.padding < 0 && index == 0)
                    || (pending.padding > 0 && index + 1 == pending.count)
                {
                    pending.padding
                } else {
                    0
                };
                DemuxEvent::Packet(Packet {
                    data: &self.block_data[pending.ranges[index].clone()],
                    track_number: track.number,
                    timestamp_ns,
                    block_timestamp_ns: pending.timestamp,
                    lace_index: index as u16,
                    lace_count: pending.count as u16,
                    block_offset: pending.header.offset,
                    default_duration_ns: track.default_duration_ns,
                    block_duration_ns: pending.duration,
                    discard_padding_ns,
                    keyframe: pending.keyframe,
                    discardable: pending.discardable,
                    invisible: pending.invisible,
                })
            }
        })
    }

    fn advance(&mut self) -> Result<Action, DemuxError> {
        if self
            .packets
            .as_ref()
            .is_some_and(|packets| packets.next < packets.count)
        {
            return Ok(Action::Packet);
        }
        self.packets = None;
        let mut scratch = [0; 8192];
        for _ in 0..self.limits.max_events_per_poll {
            match self.reader.poll(&mut scratch)? {
                stream::Event::Start { header, kind } => self.start(header, kind)?,
                stream::Event::Data { len } => {
                    let leaf = self.leaf.ok_or(DemuxError::Invalid("data without leaf"))?;
                    let output = if matches!(leaf.id.value(), 0xa1 | 0xa3) {
                        &mut self.block_data
                    } else {
                        &mut self.metadata
                    };
                    output
                        .try_reserve(len)
                        .map_err(|_| DemuxError::Limit("allocation"))?;
                    output.extend_from_slice(&scratch[..len]);
                }
                stream::Event::End { header, .. } => {
                    if let Some(action) = self.end(header)? {
                        return Ok(action);
                    }
                }
                stream::Event::NeedInput => return Ok(Action::NeedInput),
                stream::Event::Yield => return Ok(Action::Yield),
                stream::Event::SourceEof => {
                    if !self.segment_done || !self.info_done || !self.tracks_done {
                        return Err(DemuxError::Invalid("missing required document structure"));
                    }
                    return Ok(Action::End(self.completion));
                }
            }
            if self
                .packets
                .as_ref()
                .is_some_and(|packets| packets.next < packets.count)
            {
                return Ok(Action::Packet);
            }
        }
        Ok(Action::Yield)
    }

    fn start(&mut self, header: Header, kind: Kind) -> Result<(), DemuxError> {
        let id = header.id.value();
        if !self.header_done && self.depth == 0 && id != EBML {
            return Err(DemuxError::Invalid("document must start with EBML header"));
        }
        if id == EBML && self.header_done {
            return Err(DemuxError::Unsupported("chained EBML documents"));
        }
        if id == SEGMENT && self.segment_seen {
            return Err(DemuxError::Unsupported("multiple Segments"));
        }
        let index = schema::ENTRIES
            .iter()
            .position(|entry| entry.id == id)
            .ok_or(DemuxError::Invalid("internal schema ID"))?;
        let mask = 1u128
            .checked_shl(index as u32)
            .ok_or(DemuxError::Invalid("internal schema size"))?;
        let seen = if self.depth == 0 {
            &mut self.root_seen
        } else {
            &mut self.contexts[self.depth - 1].seen
        };
        if *seen & mask != 0 && !schema::repeatable(id) {
            return Err(DemuxError::Invalid("duplicate singleton element"));
        }
        *seen |= mask;
        if matches!(id, 0xbf | 0x6d80 | 0x41e4 | 0xe2 | 0xa4) {
            return Err(DemuxError::Unsupported(match id {
                0xbf => "CRC-32 verification",
                0x6d80 => "content compression or encryption",
                0x41e4 => "block additions",
                0xe2 => "track operations",
                _ => "codec state changes",
            }));
        }
        if matches!(id, EBML | INFO | TRACKS) {
            let ElementSize::Known(size) = header.size else {
                return Err(DemuxError::Invalid("unknown metadata master size"));
            };
            self.metadata_size = self
                .metadata_size
                .checked_add(size)
                .ok_or(DemuxError::Limit("metadata size"))?;
            if self.metadata_size > self.limits.max_metadata_size {
                return Err(DemuxError::Limit("metadata size"));
            }
        }
        match id {
            SEGMENT => {
                self.segment_seen = true;
                self.completion = if matches!(header.size, ElementSize::Known(_)) {
                    Completion::FiniteSegment
                } else {
                    Completion::UnknownSizeSegment
                };
            }
            0xae => {
                if self.tracks.len() == self.limits.max_tracks {
                    return Err(DemuxError::Limit("track count"));
                }
                self.building_track = TrackState {
                    track: Some(Track {
                        number: 0,
                        uid: 0,
                        track_type: 0,
                        codec_id: String::new(),
                        codec_private: Vec::new(),
                        codec_delay_ns: 0,
                        seek_preroll_ns: 0,
                        max_block_addition_id: 0,
                        default_duration_ns: None,
                        lacing_allowed: true,
                        enabled: true,
                        audio: None,
                    }),
                    output_frequency: None,
                };
            }
            0xe1 => {
                self.track_mut()?.audio = Some(AudioParameters {
                    sampling_frequency: 8000.0,
                    output_sampling_frequency: 8000.0,
                    channels: 1,
                    bit_depth: None,
                });
            }
            CLUSTER => {
                if !self.info_done || !self.tracks_done {
                    return Err(DemuxError::Unsupported(
                        "Info and Tracks must precede Clusters for sequential audio",
                    ));
                }
                self.cluster_timestamp = None;
            }
            0xa0 => {
                self.group = Some(Group::default());
            }
            0x75a1 => {
                // A BlockGroup may place additions before Block. Retain the
                // fact until its track is known; framing checks the finite
                // opaque boundary and skip budget without allocating payload.
                self.group
                    .as_mut()
                    .ok_or(DemuxError::Invalid("additions outside group"))?
                    .additions = true;
            }
            0xa1 | 0xa3 => {
                if self.cluster_timestamp.is_none() {
                    return Err(DemuxError::Unsupported(
                        "Cluster Timestamp must precede blocks",
                    ));
                }
                let ElementSize::Known(size) = header.size else {
                    return Err(DemuxError::Invalid("unknown block size"));
                };
                if size > self.limits.block.max_block_size as u64 {
                    return Err(DemuxError::Limit("block size"));
                }
                self.block_data.clear();
                if id == 0xa1 {
                    self.group
                        .as_mut()
                        .ok_or(DemuxError::Invalid("Block outside group"))?
                        .header = Some(header);
                }
            }
            _ => (),
        }
        if matches!(kind, Kind::Master { .. }) {
            let context = self
                .contexts
                .get_mut(self.depth)
                .ok_or(DemuxError::Limit("depth"))?;
            *context = Context { id, seen: 0 };
            self.depth += 1;
        } else if kind != Kind::Skip {
            self.metadata.clear();
            self.leaf = Some(header);
        }
        Ok(())
    }

    fn end(&mut self, header: Header) -> Result<Option<Action>, DemuxError> {
        let id = header.id.value();
        if self.leaf.is_some_and(|leaf| leaf.offset == header.offset) {
            self.leaf = None;
            if id == 0xa3 {
                self.prepare_packets(header, true)?;
            } else if id != 0xa1 {
                self.scalar(id)?;
            }
            return Ok(None);
        }
        let master = schema::ENTRIES
            .iter()
            .find(|entry| entry.id == id)
            .is_some_and(|entry| matches!(entry.kind, Kind::Master { .. }));
        if !master {
            return Ok(None);
        }
        let context = self
            .depth
            .checked_sub(1)
            .map(|index| self.contexts[index])
            .ok_or(DemuxError::Invalid("unbalanced context"))?;
        if context.id != id {
            return Err(DemuxError::Invalid("mismatched context"));
        }
        self.depth -= 1;
        match id {
            EBML => {
                let doc_type = self
                    .header
                    .doc_type
                    .as_deref()
                    .ok_or(DemuxError::Invalid("missing DocType"))?;
                if !matches!(doc_type, "webm" | "matroska") {
                    return Err(DemuxError::Unsupported("DocType"));
                }
                if self.header.version == 0
                    || self.header.doc_version == 0
                    || self.header.doc_read_version == 0
                    || self.header.read_version == 0
                    || self.header.doc_read_version > self.header.doc_version
                    || self.header.read_version > self.header.version
                {
                    return Err(DemuxError::Invalid("invalid EBML versions"));
                }
                if self.header.version != 1
                    || self.header.read_version != 1
                    || self.header.doc_read_version > 4
                {
                    return Err(DemuxError::Unsupported("EBML or DocType read version"));
                }
                if self.header.id_width != 4 || !(1..=8).contains(&self.header.size_width) {
                    return Err(DemuxError::Invalid("Matroska integer width declarations"));
                }
                self.reader
                    .set_width_limits(4, self.header.size_width as u8)?;
                self.header_done = true;
            }
            INFO => {
                for required in [0x4d80, 0x5741] {
                    if !contains(context, required) {
                        return Err(DemuxError::Invalid("missing Info application metadata"));
                    }
                }
                self.info_done = true;
            }
            0xae => {
                for required in [0xd7, 0x73c5, 0x83, 0x86] {
                    if !contains(context, required) {
                        return Err(DemuxError::Invalid("missing required TrackEntry field"));
                    }
                }
                let mut track = self
                    .building_track
                    .track
                    .take()
                    .ok_or(DemuxError::Invalid("missing track state"))?;
                if self
                    .tracks
                    .iter()
                    .any(|other| other.number == track.number || other.uid == track.uid)
                {
                    return Err(DemuxError::Invalid("duplicate track number or UID"));
                }
                if track.track_type == 2 {
                    let audio = track
                        .audio
                        .as_mut()
                        .ok_or(DemuxError::Invalid("missing Audio settings"))?;
                    audio.output_sampling_frequency = self
                        .building_track
                        .output_frequency
                        .unwrap_or(audio.sampling_frequency);
                } else if track.audio.is_some() {
                    return Err(DemuxError::Invalid("Audio settings on nonaudio track"));
                }
                self.tracks
                    .try_reserve(1)
                    .map_err(|_| DemuxError::Limit("allocation"))?;
                self.tracks.push(track);
            }
            TRACKS => {
                self.selected = self.tracks.iter().position(|track| {
                    track.track_type == 2
                        && track.enabled
                        && match self.selection {
                            AudioSelection::First => true,
                            AudioSelection::Number(number) => track.number == number,
                        }
                });
                if self.selected.is_none() {
                    return Err(DemuxError::AudioTrackUnavailable);
                }
                self.tracks_done = true;
                return Ok(Some(Action::Tracks));
            }
            CLUSTER => {
                if self.cluster_timestamp.is_none() {
                    return Err(DemuxError::Invalid("missing Cluster Timestamp"));
                }
            }
            0xa0 => {
                let header = self
                    .group
                    .as_ref()
                    .and_then(|group| group.header)
                    .ok_or(DemuxError::Invalid("missing Block in BlockGroup"))?;
                self.prepare_packets(header, false)?;
                self.group = None;
            }
            SEGMENT => {
                self.segment_done = true;
            }
            _ => (),
        }
        Ok(None)
    }

    fn track_mut(&mut self) -> Result<&mut Track, DemuxError> {
        self.building_track
            .track
            .as_mut()
            .ok_or(DemuxError::Invalid("track field outside TrackEntry"))
    }
    fn audio_mut(&mut self) -> Result<&mut AudioParameters, DemuxError> {
        self.track_mut()?
            .audio
            .as_mut()
            .ok_or(DemuxError::Invalid("audio field outside Audio"))
    }

    fn scalar(&mut self, id: u32) -> Result<(), DemuxError> {
        match id {
            0x4286 => self.header.version = uint_default(&self.metadata, 1)?,
            0x42f7 => self.header.read_version = uint_default(&self.metadata, 1)?,
            0x42f2 => self.header.id_width = uint_default(&self.metadata, 4)?,
            0x42f3 => self.header.size_width = uint_default(&self.metadata, 8)?,
            0x4287 => self.header.doc_version = uint_default(&self.metadata, 1)?,
            0x4285 => self.header.doc_read_version = uint_default(&self.metadata, 1)?,
            0x4282 => {
                validate_string(&self.metadata, true)?;
                trim_null(&mut self.metadata);
                self.header.doc_type = Some(
                    String::from_utf8(std::mem::take(&mut self.metadata))
                        .map_err(|_| DemuxError::Invalid("DocType ASCII"))?,
                );
            }
            0x2ad7b1 => {
                self.info.timestamp_scale_ns = if self.metadata.is_empty() {
                    1_000_000
                } else {
                    nonzero(&self.metadata)?
                }
            }
            0x4489 => {
                let value = float(&self.metadata)?;
                if value <= 0.0 {
                    return Err(DemuxError::Invalid("nonpositive Duration"));
                }
                self.info.duration_ticks = Some(value);
            }
            0x4d80 | 0x5741 => validate_string(&self.metadata, false)?,
            0xd7 => {
                let value = nonzero(&self.metadata)?;
                if value >= 1 << 56 {
                    return Err(DemuxError::Unsupported(
                        "track number cannot fit block VINT",
                    ));
                }
                self.track_mut()?.number = value;
            }
            0x73c5 => {
                let value = nonzero(&self.metadata)?;
                self.track_mut()?.uid = value;
            }
            0x83 => {
                let value = nonzero(&self.metadata)?;
                self.track_mut()?.track_type = value;
            }
            0xb9 | 0x88 | 0x55aa | 0x9c => {
                let value = uint_default(&self.metadata, u64::from(id != 0x55aa))?;
                if value > 1 {
                    return Err(DemuxError::Invalid("boolean outside 0 or 1"));
                }
                if id == 0xb9 {
                    self.track_mut()?.enabled = value == 1;
                }
                if id == 0x9c {
                    self.track_mut()?.lacing_allowed = value == 1;
                }
            }
            0x23e383 => {
                let value = nonzero(&self.metadata)?;
                self.track_mut()?.default_duration_ns = Some(value);
            }
            0x23314f => {
                if !self.metadata.is_empty() && float(&self.metadata)? != 1.0 {
                    return Err(DemuxError::Unsupported("nonunit TrackTimestampScale"));
                }
            }
            0x55ee => {
                let value = uint(&self.metadata)?;
                self.track_mut()?.max_block_addition_id = value;
            }
            0x537f => {
                if sint(&self.metadata)? != 0 {
                    return Err(DemuxError::Unsupported("nonzero TrackOffset"));
                }
            }
            0x86 => {
                validate_string(&self.metadata, true)?;
                trim_null(&mut self.metadata);
                if self.metadata.is_empty() {
                    return Err(DemuxError::Invalid("empty CodecID"));
                }
                let value = String::from_utf8(std::mem::take(&mut self.metadata))
                    .map_err(|_| DemuxError::Invalid("CodecID ASCII"))?;
                self.track_mut()?.codec_id = value;
            }
            0x63a2 => {
                let value = std::mem::take(&mut self.metadata);
                self.track_mut()?.codec_private = value;
            }
            0x56aa | 0x56bb => {
                let value = uint(&self.metadata)?;
                if id == 0x56aa {
                    self.track_mut()?.codec_delay_ns = value;
                } else {
                    self.track_mut()?.seek_preroll_ns = value;
                }
            }
            0xb5 | 0x78b5 => {
                if self.metadata.is_empty() {
                    return Ok(());
                }
                let value = float(&self.metadata)?;
                if value <= 0.0 {
                    return Err(DemuxError::Invalid("nonpositive sampling frequency"));
                }
                if id == 0xb5 {
                    self.audio_mut()?.sampling_frequency = value;
                } else {
                    self.building_track.output_frequency = Some(value);
                }
            }
            0x9f => {
                let value = if self.metadata.is_empty() {
                    1
                } else {
                    nonzero(&self.metadata)?
                };
                self.audio_mut()?.channels = value;
            }
            0x6264 => {
                let value = nonzero(&self.metadata)?;
                self.audio_mut()?.bit_depth = Some(value);
            }
            0xe7 => self.cluster_timestamp = Some(uint(&self.metadata)?),
            0xa7 | 0xab => {
                let _value = uint(&self.metadata)?;
            }
            0x9b => {
                let value = uint(&self.metadata)?;
                self.group
                    .as_mut()
                    .ok_or(DemuxError::Invalid("duration outside group"))?
                    .duration = Some(value);
            }
            0xfb => {
                let _value = sint(&self.metadata)?;
                self.group
                    .as_mut()
                    .ok_or(DemuxError::Invalid("reference outside group"))?
                    .reference = true;
            }
            0x75a2 => {
                let value = sint(&self.metadata)?;
                self.group
                    .as_mut()
                    .ok_or(DemuxError::Invalid("padding outside group"))?
                    .padding = value;
            }
            0xfa => {
                let _value = uint(&self.metadata)?;
            }
            _ => return Err(DemuxError::Invalid("unhandled scalar")),
        }
        Ok(())
    }

    fn prepare_packets(&mut self, header: Header, simple: bool) -> Result<(), DemuxError> {
        let parsed = block::parse_block(
            &self.block_data,
            if simple {
                BlockKind::SimpleBlock
            } else {
                BlockKind::Block
            },
            self.limits.block,
        )?;
        let track_index = self
            .tracks
            .iter()
            .position(|track| track.number == parsed.track_number())
            .ok_or(DemuxError::Invalid("block references unknown track"))?;
        let track = &self.tracks[track_index];
        if parsed.lacing() != Lacing::None && !track.lacing_allowed {
            return Err(DemuxError::Invalid("lacing forbidden by track"));
        }
        let cluster = self
            .cluster_timestamp
            .ok_or(DemuxError::Invalid("missing Cluster Timestamp"))?;
        let timestamp = (i128::from(cluster) + i128::from(parsed.relative_timestamp()))
            .checked_mul(i128::from(self.info.timestamp_scale_ns))
            .ok_or(DemuxError::Invalid("timestamp overflow"))?;
        // Validate presentation arithmetic even for unselected tracks.
        timestamp
            .checked_sub(i128::from(track.codec_delay_ns))
            .ok_or(DemuxError::Invalid("timestamp overflow"))?;
        if self.selected != Some(track_index) {
            return Ok(());
        }
        if self.group.as_ref().is_some_and(|group| group.additions) {
            return Err(DemuxError::Unsupported("block additions"));
        }
        let mut start =
            self.block_data.len() - parsed.frames().map(|frame| frame.len()).sum::<usize>();
        let count = parsed.frames().len();
        let mut ranges = std::array::from_fn(|_| 0..0);
        for (index, frame) in parsed.frames().enumerate() {
            let end = start + frame.len();
            ranges[index] = start..end;
            start = end;
        }
        let group = self.group.as_ref();
        let duration = match group.and_then(|group| group.duration) {
            Some(duration) => Some(
                i128::from(duration)
                    .checked_mul(i128::from(self.info.timestamp_scale_ns))
                    .ok_or(DemuxError::Invalid("duration overflow"))?,
            ),
            None => None,
        };
        // Check every future laced timestamp now, so poll's borrowing path needs
        // no fallible arithmetic and cannot expose wrapped or fabricated times.
        let presentation = timestamp
            .checked_sub(i128::from(track.codec_delay_ns))
            .ok_or(DemuxError::Invalid("timestamp overflow"))?;
        if let Some(duration) = track.default_duration_ns {
            presentation
                .checked_add((count - 1) as i128 * i128::from(duration))
                .ok_or(DemuxError::Invalid("timestamp overflow"))?;
        }
        self.packets = Some(Packets {
            ranges,
            count,
            next: 0,
            track: track_index,
            header,
            timestamp,
            duration,
            padding: group.map_or(0, |group| group.padding),
            keyframe: parsed
                .keyframe()
                .unwrap_or_else(|| !group.is_some_and(|group| group.reference)),
            discardable: parsed.discardable().unwrap_or(false),
            invisible: parsed.invisible(),
        });
        Ok(())
    }
}

fn contains(context: Context, id: u32) -> bool {
    schema::ENTRIES
        .iter()
        .position(|entry| entry.id == id)
        .is_some_and(|index| context.seen & (1u128 << index) != 0)
}
fn uint(bytes: &[u8]) -> Result<u64, DemuxError> {
    if bytes.len() > 8 {
        return Err(DemuxError::Invalid("integer exceeds eight bytes"));
    }
    Ok(bytes
        .iter()
        .fold(0, |value, &byte| (value << 8) | u64::from(byte)))
}
fn uint_default(bytes: &[u8], default: u64) -> Result<u64, DemuxError> {
    if bytes.is_empty() {
        Ok(default)
    } else {
        uint(bytes)
    }
}
fn nonzero(bytes: &[u8]) -> Result<u64, DemuxError> {
    let value = uint(bytes)?;
    if value == 0 {
        Err(DemuxError::Invalid("zero value outside element range"))
    } else {
        Ok(value)
    }
}
fn sint(bytes: &[u8]) -> Result<i64, DemuxError> {
    let value = uint(bytes)?;
    if !bytes.is_empty() && bytes.len() < 8 && bytes[0] & 0x80 != 0 {
        Ok((value | (u64::MAX << (8 * bytes.len()))) as i64)
    } else {
        Ok(value as i64)
    }
}
fn float(bytes: &[u8]) -> Result<f64, DemuxError> {
    let value = match bytes.len() {
        0 => 0.0,
        4 => f64::from(f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
        8 => f64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]),
        _ => {
            return Err(DemuxError::Invalid(
                "float must be zero, four or eight bytes",
            ));
        }
    };
    if !value.is_finite() {
        return Err(DemuxError::Invalid("nonfinite float"));
    }
    Ok(value)
}
fn validate_string(bytes: &[u8], ascii: bool) -> Result<(), DemuxError> {
    let prefix = bytes.split(|&byte| byte == 0).next().unwrap_or(&[]);
    if (ascii && !prefix.is_ascii()) || std::str::from_utf8(prefix).is_err() {
        return Err(DemuxError::Invalid("invalid string encoding"));
    }
    Ok(())
}
fn trim_null(bytes: &mut Vec<u8>) {
    if let Some(index) = bytes.iter().position(|&byte| byte == 0) {
        bytes.truncate(index);
    }
}
