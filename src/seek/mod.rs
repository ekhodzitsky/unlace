//! Bounded container seeking over immutable positional input.
//!
//! Seeking attests element boundaries and selected-track access points, not
//! decoder state or the validity/completeness of skipped media.
mod index;

use crate::demux::{
    AudioSelection, Completion, DemuxError, DemuxEvent, DemuxLimits, Demuxer, Packet, SegmentInfo,
    Track,
};
use crate::source::{RangeReader, ReadAt, ReadLimits, ReadReport, SourceError};
use std::fmt;
use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, Ordering};

/// Independent bounds for index storage, preparation and seek scans.
#[derive(Clone, Copy, Debug)]
pub struct SeekLimits {
    /// Positional read budget for each prepare, seek or packet poll call.
    pub read: ReadLimits,
    /// Maximum retained Segment children and cue entries (each separately).
    /// Each catalogue entry includes bounded fallback summary storage, even
    /// when a chosen-cue seek does not use it.
    pub max_entries: usize,
    /// Aggregate finite Cues/SeekHead payload bytes.
    pub max_index_bytes: u64,
    /// Aggregate parsed index element count.
    pub max_index_elements: usize,
    /// Maximum track positions in one CuePoint; bounds duplicate-check work.
    pub max_positions_per_cue: usize,
    /// Maximum boundary/index steps in one preparation call.
    pub max_prepare_steps: usize,
    /// Maximum headers inspected or charged from reused validation in a single
    /// seek, including fallback scans.
    pub max_scan_headers: u64,
}
impl Default for SeekLimits {
    fn default() -> Self {
        Self {
            read: ReadLimits::default(),
            max_entries: 65_536,
            max_index_bytes: 4 * 1024 * 1024,
            max_index_elements: 1_000_000,
            max_positions_per_cue: 64,
            max_prepare_steps: 256,
            max_scan_headers: 1_000_000,
        }
    }
}

/// Seek failures. Unavailable ranges, cancellation and work refusals do not
/// commit a new packet position. Invalid/unsupported indexes poison this reader.
#[derive(Debug)]
pub enum SeekError {
    /// Positional input error, including resumable NeedRange or work Limit.
    Source(SourceError),
    /// Sequential metadata/packet failure.
    Demux(DemuxError),
    /// Invalid container/index structure at an absolute offset.
    Invalid {
        /// Offending header or advertised destination.
        offset: u64,
        /// Structural reason.
        reason: &'static str,
    },
    /// Explicitly unsupported index capability.
    Unsupported(&'static str),
    /// Invalid limits or an operation attempted before preparation.
    Configuration,
    /// Requested timestamp minus preroll cannot be represented. The reader and
    /// active packet position remain usable for another request.
    RequestOverflow,
    /// An index/scan/storage bound was reached.
    Limit(&'static str),
    /// Cooperative cancellation; does not interrupt a provider method.
    Cancelled,
    /// No selected-track block exists in the scanned source.
    NoAccessPoint,
    /// Earlier fatal parsing/index error; reconstruct to try another source.
    Failed,
}
impl fmt::Display for SeekError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(f),
            Self::Demux(error) => error.fmt(f),
            Self::Invalid { offset, reason } => {
                write!(f, "invalid seek structure at {offset}: {reason}")
            }
            Self::Unsupported(reason) => write!(f, "unsupported seek feature: {reason}"),
            Self::Configuration => f.write_str("invalid seek configuration or reader not prepared"),
            Self::RequestOverflow => {
                f.write_str("requested preroll target exceeds timestamp range")
            }
            Self::Limit(reason) => write!(f, "seek resource limit: {reason}"),
            Self::Cancelled => f.write_str("seek cancelled"),
            Self::NoAccessPoint => f.write_str("no selected-track access point"),
            Self::Failed => f.write_str("seek reader failed"),
        }
    }
}
impl std::error::Error for SeekError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Demux(error) => Some(error),
            _ => None,
        }
    }
}
impl From<SourceError> for SeekError {
    fn from(error: SourceError) -> Self {
        Self::Source(error)
    }
}
impl From<DemuxError> for SeekError {
    fn from(error: DemuxError) -> Self {
        Self::Demux(error)
    }
}

/// Preparation progress; repeat Yield until Ready. Missing source ranges are
/// returned as SeekError::Source(SourceError::NeedRange), preserving progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preparation {
    /// Validated metadata, boundaries and bounded indexes are ready.
    Ready,
    /// Work budget exhausted; preparation retains completed steps.
    Yield,
}

/// Request in the same signed nanosecond timeline as Packet::timestamp_ns.
#[derive(Clone, Copy, Debug)]
pub struct SeekRequest {
    /// Desired presentation position, already accounting for CodecDelay.
    pub timestamp_ns: i128,
    /// Caller-required preroll. The effective value is at least SeekPreRoll.
    pub preroll_ns: u64,
}
/// Why a full bounded header scan was used instead of a selected-track cue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
    /// No Cues element.
    MissingCues,
    /// Cues contain no entry for the selected track.
    NoTrackCues,
    /// No selected-track cue precedes the preroll target.
    BeforeFirstCue,
}

/// Actual committed container access point. Decode/preroll recovery is the
/// consumer's responsibility; no encoded packets are trimmed by this operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeekResult {
    /// Requested presentation timestamp.
    pub requested_ns: i128,
    /// Raw declared SeekPreRoll.
    pub declared_preroll_ns: u64,
    /// Maximum of requested and declared preroll; no codec recommendation added.
    pub required_preroll_ns: u64,
    /// Requested timestamp minus effective preroll.
    pub preroll_target_ns: i128,
    /// Actual selected-track first block timestamp in the chosen Cluster.
    pub actual_timestamp_ns: i128,
    /// Whether the actual start is at or before the preroll target. This is a
    /// timestamp comparison, not evidence of decoder readiness or valid packets.
    pub starts_before_preroll_target: bool,
    /// Absolute Cluster header boundary where packet reading restarts.
    pub cluster_offset: u64,
    /// Absolute Block/SimpleBlock header for the first selected packet.
    pub block_offset: u64,
    /// Selected TrackNumber, never a track chosen by the index.
    pub track_number: u64,
    /// Validated cue timestamp when used (before CodecDelay, in Segment ticks).
    pub cue_time_ticks: Option<u64>,
    /// Explicit fallback reason, if a cue was not used.
    pub fallback: Option<Fallback>,
    /// Bytes before this Cluster payload reading will not verify. Header/index
    /// inspection is not packet validation, including inside this extent.
    pub skipped_prefix: crate::source::ByteExtent,
}

/// Completion of the suffix actually read after the last seek. Does not verify
/// skipped media, producer intent or decoder state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuffixCompletion {
    /// Container boundary/EOF evidence observed for the suffix.
    pub observed: Completion,
    /// Initial Cluster of this suffix, even if no earlier packets were emitted.
    pub from_cluster_offset: u64,
}
/// Packet events after an explicit successful seek.
pub enum SeekEvent<'a> {
    /// Original borrowed encoded packet and exact core metadata.
    Packet(Packet<'a>),
    /// Bounded packet processing yielded. Call again to continue.
    Yield,
    /// Suffix ended; unread/skipped media remains unverified.
    End(SuffixCompletion),
}

struct Input<S> {
    source: RangeReader<S>,
    position: u64,
    last: Option<SourceError>,
}
impl<S: ReadAt> Read for Input<S> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.source.read_at(self.position, buffer) {
            Ok(len) => {
                self.position += len as u64;
                Ok(len)
            }
            Err(error) => {
                let kind = match &error {
                    SourceError::NeedRange(_) | SourceError::Limit => io::ErrorKind::WouldBlock,
                    SourceError::Io { error, .. } if error.kind() == io::ErrorKind::Interrupted => {
                        return Err(io::ErrorKind::Interrupted.into());
                    }
                    _ => io::ErrorKind::Other,
                };
                self.last = Some(error);
                Err(io::Error::new(kind, "positional source failure"))
            }
        }
    }
}

/// Seekable selected-audio reader with immutable-source validation.
///
/// Prepare first, then explicitly seek and poll packets through suffix End.
/// Index opening and subsequent seek I/O are reported separately per call.
/// A successful fallback retains verified first-block summaries for this track
/// and immutable source. Later fallbacks reuse them with the original scan-work
/// charge and fresh snapshot checks, without re-reading those prefixes. Chosen
/// cues still require their own block/time/relative-position attestation.
/// A failed/cancelled seek leaves the prior packet cursor and buffered packets
/// intact. Fatal index/container errors prevent further operations. Generic
/// blocking providers cannot be interrupted; cancellation is checked between I/O.
pub struct SeekReader<S> {
    demux: Demuxer<Input<S>>,
    limits: SeekLimits,
    index: Option<index::Index>,
    prepared: bool,
    from: Option<u64>,
    failed: bool,
}
impl<S: ReadAt> SeekReader<S> {
    /// Own a positional provider. No source payload is read by construction.
    pub fn new(
        source: S,
        selection: AudioSelection,
        demux_limits: DemuxLimits,
        limits: SeekLimits,
    ) -> Result<Self, SeekError> {
        if limits.max_entries == 0
            || limits.max_index_bytes == 0
            || limits.max_index_elements == 0
            || limits.max_positions_per_cue == 0
            || limits.max_prepare_steps == 0
            || limits.max_scan_headers == 0
        {
            return Err(SeekError::Configuration);
        }
        let input = Input {
            source: RangeReader::new(source, limits.read)?,
            position: 0,
            last: None,
        };
        Ok(Self {
            demux: Demuxer::new(input, selection, demux_limits)?,
            limits,
            index: None,
            prepared: false,
            from: None,
            failed: false,
        })
    }
    /// Finalized tracks. No packets have been consumed by preparation.
    pub fn tracks(&self) -> &[Track] {
        self.demux.tracks()
    }
    /// Finalized Segment timing, when available.
    pub fn info(&self) -> Option<SegmentInfo> {
        self.demux.info()
    }
    /// Actual read work for the last call. Preparation cost is not hidden in
    /// later seek reports; callers must retain/aggregate reports they need.
    pub fn read_report(&self) -> &ReadReport {
        self.demux.source().source.report()
    }
    fn begin(&mut self, cancel: &AtomicBool) -> Result<(), SeekError> {
        if self.failed {
            return Err(SeekError::Failed);
        }
        cancelled(cancel)?;
        self.demux.source_mut().last = None;
        self.demux.source_mut().source.begin_operation()?;
        Ok(())
    }
    fn finish<T>(&mut self, result: Result<T, SeekError>) -> Result<T, SeekError> {
        if matches!(
            &result,
            Err(SeekError::Invalid { .. } | SeekError::Unsupported(_) | SeekError::Demux(_))
        ) {
            self.failed = true;
        }
        result
    }
    /// Advance metadata, boundary and index preparation without discarding a
    /// first packet. NeedRange and cancellation retain completed preparation.
    pub fn prepare(&mut self, cancel: &AtomicBool) -> Result<Preparation, SeekError> {
        self.begin(cancel)?;
        let result = self.prepare_inner(cancel);
        self.finish(result)
    }
    fn prepare_inner(&mut self, cancel: &AtomicBool) -> Result<Preparation, SeekError> {
        let ready = self.demux.prepare_metadata();
        if ready.is_err() {
            self.failed = true;
        }
        if let Some(error) = self.demux.source_mut().last.take() {
            return Err(error.into());
        }
        if !ready? {
            cancelled(cancel)?;
            return Ok(Preparation::Yield);
        }
        if self.index.is_none() {
            let (segment, size_width, _) =
                self.demux.seek_metadata().ok_or(SeekError::Configuration)?;
            self.index = Some(index::Index::new(
                segment,
                size_width,
                self.demux.source_mut().source.snapshot().length,
            )?);
        }
        let input = self.demux.source_mut();
        let index = self.index.as_mut().ok_or(SeekError::Configuration)?;
        let status = index.prepare(&mut input.source, self.limits, cancel)?;
        if status == Preparation::Ready {
            index.validate_tracks(self.demux.tracks())?;
            cancelled(cancel)?;
            self.prepared = true;
        }
        Ok(status)
    }
    /// Choose and validate an access point and then atomically replace the
    /// active packet position. The first packet may precede the requested time.
    pub fn seek(
        &mut self,
        request: SeekRequest,
        cancel: &AtomicBool,
    ) -> Result<SeekResult, SeekError> {
        self.begin(cancel)?;
        let result = self.seek_inner(request, cancel);
        self.finish(result)
    }
    fn seek_inner(
        &mut self,
        request: SeekRequest,
        cancel: &AtomicBool,
    ) -> Result<SeekResult, SeekError> {
        if !self.prepared {
            return Err(SeekError::Configuration);
        }
        let (_, _, track) = self.demux.seek_metadata().ok_or(SeekError::Configuration)?;
        let timing = index::Timing {
            track: track.number,
            delay: track.codec_delay_ns,
            scale: self
                .demux
                .info()
                .ok_or(SeekError::Configuration)?
                .timestamp_scale_ns,
            preroll: track.seek_preroll_ns,
        };
        let cache_track = timing.track;
        let index = self
            .index
            .as_mut()
            .filter(|index| index.ready())
            .ok_or(SeekError::Configuration)?;
        let (result, cache_candidate) = index.locate(
            &mut self.demux.source_mut().source,
            timing,
            request,
            self.limits,
            cancel,
        )?;
        cancelled(cancel)?;
        self.demux.source_mut().source.check_snapshot()?;
        self.demux.restart_cluster(result.cluster_offset)?;
        self.demux.source_mut().position = result.cluster_offset;
        self.from = Some(result.cluster_offset);
        // No fallible operation may follow committing the verified summaries.
        if cache_candidate {
            index.commit_fallback(cache_track);
        }
        Ok(result)
    }
    /// Read a suffix packet. NeedRange retains exact core continuation. Limit
    /// refusal is resumable by calling poll again with a fresh bounded operation.
    pub fn poll(&mut self, cancel: &AtomicBool) -> Result<SeekEvent<'_>, SeekError> {
        self.begin(cancel)?;
        let from = self.from.ok_or(SeekError::Configuration)?;
        let action = self.demux.poll_action();
        if action.is_err() {
            self.failed = true;
        }
        if let Some(error) = self.demux.source_mut().last.take() {
            return Err(error.into());
        }
        let action = action?;
        cancelled(cancel)?;
        self.demux.source_mut().source.check_snapshot()?;
        match self.demux.event(action)? {
            DemuxEvent::Packet(packet) => Ok(SeekEvent::Packet(packet)),
            DemuxEvent::Yield | DemuxEvent::TracksReady => Ok(SeekEvent::Yield),
            DemuxEvent::NeedInput => Err(SeekError::Configuration),
            DemuxEvent::End(observed) => Ok(SeekEvent::End(SuffixCompletion {
                observed,
                from_cluster_offset: from,
            })),
        }
    }
    /// Recover the provider, discarding all index/packet continuation state.
    pub fn into_inner(self) -> S {
        self.demux.into_inner().source.into_inner()
    }
}

fn cancelled(cancel: &AtomicBool) -> Result<(), SeekError> {
    if cancel.load(Ordering::Relaxed) {
        Err(SeekError::Cancelled)
    } else {
        Ok(())
    }
}
