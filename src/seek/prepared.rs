use super::*;

/// One owned immutable provider with retained metadata and verified seek state.
///
/// Preparation includes a complete selected-track fallback scan. Its cost and
/// storage remain owned here between exclusive consumer sessions. The original
/// provider stays alive: this is in-process reuse, not a serialized index or a
/// way to attach verification to another provider with matching snapshot fields.
/// Recovering the provider consumes this owner and discards verification.
///
/// Each prepare/open/poll call retains the configured per-operation budgets.
/// Preparation can refuse the full fallback work even when ordinary metadata
/// preparation would succeed. No partial fallback cache is published on failure.
/// Provider snapshot methods must still be bounded and revisions immutable.
///
/// ```
/// use std::sync::atomic::AtomicBool;
/// use unlace::{demux::{AudioSelection, DemuxLimits}, source::ReadAt,
///     seek::{PreparedSource, Preparation, SeekLimits, SeekRequest, SeekEvent, SeekError}};
/// fn consume<S: ReadAt>(source: S) -> Result<(), SeekError> {
///     let cancel = AtomicBool::new(false);
///     let mut prepared = PreparedSource::new(source, AudioSelection::First,
///         DemuxLimits::default(), SeekLimits::default())?;
///     while prepared.prepare(&cancel)? != Preparation::Ready {}
///     for timestamp_ns in [0, 1_000_000_000, 0] {
///         let mut session = prepared.open(SeekRequest { timestamp_ns, preroll_ns: 0 }, &cancel)?;
///         let actual_start = session.start();
///         loop {
///             match session.poll(&cancel)? {
///                 SeekEvent::Packet(packet) => { let encoded_bytes = packet.data; }
///                 SeekEvent::Yield => (),
///                 SeekEvent::End(_) => break,
///             }
///         }
///     }
///     Ok(())
/// }
/// ```
///
/// Sessions borrow exclusively; consumers cannot share one packet cursor:
///
/// ```compile_fail
/// use std::sync::atomic::AtomicBool;
/// use unlace::{source::ReadAt, seek::{PreparedSource, SeekRequest, SeekError}};
/// fn overlap<S: ReadAt>(source: &mut PreparedSource<S>, request: SeekRequest,
///     cancel: &AtomicBool) -> Result<(), SeekError> {
///     let mut first = source.open(request, cancel)?;
///     let _second = source.open(request, cancel)?;
///     let _event = first.poll(cancel)?;
///     Ok(())
/// }
/// ```
/// Recovering the provider cannot overlap an active session:
///
/// ```compile_fail
/// use std::sync::atomic::AtomicBool;
/// use unlace::{source::ReadAt, seek::{PreparedSource, SeekRequest, SeekError}};
/// fn detach<S: ReadAt>(mut source: PreparedSource<S>, request: SeekRequest,
///     cancel: &AtomicBool) -> Result<(), SeekError> {
///     let mut session = source.open(request, cancel)?;
///     let _provider = source.into_inner();
///     let _event = session.poll(cancel)?;
///     Ok(())
/// }
/// ```
pub struct PreparedSource<S> {
    reader: SeekReader<S>,
    ready: bool,
}
impl<S: ReadAt> PreparedSource<S> {
    /// Capture the original provider and limits without reading source payloads.
    pub fn new(
        source: S,
        selection: AudioSelection,
        demux_limits: DemuxLimits,
        limits: SeekLimits,
    ) -> Result<Self, SeekError> {
        Ok(Self {
            reader: SeekReader::new(source, selection, demux_limits, limits)?,
            ready: false,
        })
    }

    /// Prepare metadata/catalogue, then warm the complete fallback summaries.
    ///
    /// Catalogue readiness yields before warming, so one public call never
    /// silently resets the read budget between two internal operations. The
    /// final phase has the existing full-seek read/scan bounds; failures retain
    /// no reusable partial fallback progress. Retry only under caller bounds.
    /// Empty selected tracks can return NoAccessPoint. Chosen-cue-only sources
    /// also pay for this full fallback scan, unlike ordinary SeekReader::prepare.
    /// Ready calls still check source identity and cancellation. No packets are
    /// consumed; a session always needs its own successful opening request.
    pub fn prepare(&mut self, cancel: &AtomicBool) -> Result<Preparation, SeekError> {
        if !self.reader.prepared {
            self.reader.prepare(cancel)?;
            return Ok(Preparation::Yield);
        }
        if !self.ready {
            let (_, _, track) = self
                .reader
                .demux
                .seek_metadata()
                .ok_or(SeekError::Configuration)?;
            // All supported container timestamps lie above this target. Adding
            // a u64 preroll to i128::MIN is representable, even at u64::MAX.
            let request = SeekRequest {
                timestamp_ns: i128::MIN + i128::from(track.seek_preroll_ns),
                preroll_ns: 0,
            };
            self.reader.seek(request, cancel)?;
            self.ready = true;
        } else {
            self.reader.begin(cancel)?;
            self.reader.demux.source_mut().source.check_snapshot()?;
            cancelled(cancel)?;
        }
        Ok(Preparation::Ready)
    }

    /// Open an exclusive, positioned session after checking the original source.
    ///
    /// Preparation must have returned Ready. A failed opening returns no session
    /// and cannot expose the prior consumer's buffered packets. Chosen cues still
    /// require their normal attestation. No source handle/identity is replaceable.
    pub fn open(
        &mut self,
        request: SeekRequest,
        cancel: &AtomicBool,
    ) -> Result<SeekSession<'_, S>, SeekError> {
        if !self.ready {
            self.reader.begin(cancel)?;
            return Err(SeekError::Configuration);
        }
        let start = self.reader.seek(request, cancel)?;
        Ok(SeekSession {
            reader: &mut self.reader,
            start,
        })
    }

    /// Finalized track metadata retained by this source owner.
    pub fn tracks(&self) -> &[Track] {
        self.reader.tracks()
    }
    /// Finalized Segment timing, when available.
    pub fn info(&self) -> Option<SegmentInfo> {
        self.reader.info()
    }
    /// Actual source read work for the latest bounded operation. A failed
    /// operation reset retains the prior report, as on the ordinary seek reader.
    pub fn read_report(&self) -> &ReadReport {
        self.reader.read_report()
    }
    /// Recover the original provider, discarding all verification and cursors.
    pub fn into_inner(self) -> S {
        self.reader.into_inner()
    }
}

/// An exclusive consumer session positioned by a successful prepared opening.
///
/// Dropping a session performs no I/O and preserves the owner's preparation,
/// verified summaries and fatal/source latches. The next opening must succeed
/// before another session can observe packets. Packet borrows cannot outlive the
/// session or overlap another opening. Sessions do not close/reopen the provider.
pub struct SeekSession<'a, S> {
    reader: &'a mut SeekReader<S>,
    start: SeekResult,
}
impl<S: ReadAt> SeekSession<'_, S> {
    /// Actual access point and declared/requested preroll for this session.
    pub fn start(&self) -> SeekResult {
        self.start
    }
    /// Retained metadata for the original source and fixed selected track.
    pub fn tracks(&self) -> &[Track] {
        self.reader.tracks()
    }
    /// Finalized Segment timing.
    pub fn info(&self) -> Option<SegmentInfo> {
        self.reader.info()
    }
    /// Actual source read work for the latest opening or poll operation.
    pub fn read_report(&self) -> &ReadReport {
        self.reader.read_report()
    }
    /// Read supported suffix packets through explicit completion. NeedRange,
    /// work bounds and cancellation follow the ordinary seek reader contract.
    pub fn poll(&mut self, cancel: &AtomicBool) -> Result<SeekEvent<'_>, SeekError> {
        self.reader.poll(cancel)
    }
}
