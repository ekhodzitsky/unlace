//! Bounded positional reads of one immutable source snapshot.
//!
//! Input infrastructure for random access, not a container seek API. Offsets
//! and read extents do not attest element boundaries or packet validity.
//!
//! ```
//! use std::io::Cursor;
//! use unlace::source::{RangeReader, ReadLimits, SeekSource, ByteExtent};
//!
//! let source = SeekSource::new(Cursor::new(b"abcdef"), 1);
//! let mut input = RangeReader::new(source, ReadLimits::default())?;
//! let mut bytes = [0; 2];
//! assert_eq!(input.read_at(4, &mut bytes)?, 2);
//! assert_eq!(&bytes, b"ef");
//! assert_eq!(input.read_at(0, &mut bytes)?, 2);
//! assert_eq!(&bytes, b"ab");
//! assert_eq!(input.report().bytes, 4);
//! assert_eq!(input.report().extents, [
//!     ByteExtent { start: 4, end: 6 }, ByteExtent { start: 0, end: 2 }
//! ]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};

/// Identity of one immutable byte snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// Complete source length, not merely the currently cached prefix.
    pub length: u64,
    /// Provider-defined revision. Any byte replacement must change this token.
    pub revision: u128,
}

/// Positional input supplied by a file or an external range cache.
///
/// Unlace performs no networking. A provider must keep bytes immutable within
/// each revision and must never reuse a revision for different bytes. Snapshot
/// checks are a provider contract, not cryptographic authentication. Cached
/// availability may grow without changing the snapshot; source content may not.
/// All methods, including snapshot checks, must return in bounded application
/// time if the caller requires bounded cancellation latency.
pub trait ReadAt {
    /// Return complete length and revision. This should be local and cheap.
    fn snapshot(&mut self) -> io::Result<Snapshot>;

    /// Read at an absolute offset, independently of previous cursor position.
    ///
    /// Short reads and Interrupted are allowed. WouldBlock means the requested
    /// range is unavailable for now; zero on a nonempty request means final EOF.
    /// An error must not return partial progress: return the byte count first.
    fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<usize>;
}

/// Adapter for an owned Read + Seek cursor kept immutable by the caller.
///
/// Length changes are detectable. Same-length external writes cannot be detected
/// by these generic traits: use a version-aware ReadAt provider when immutability
/// cannot be ensured. No bytes are loaded merely to construct the adapter.
pub struct SeekSource<R> {
    inner: R,
    revision: u128,
}
impl<R: Read + Seek> SeekSource<R> {
    /// Own a cursor and its application revision token. Its position may change
    /// even during snapshot checks; every subsequent read seeks absolutely.
    pub fn new(inner: R, revision: u128) -> Self {
        Self { inner, revision }
    }
    /// Recover the cursor at its actual last I/O position.
    pub fn into_inner(self) -> R {
        self.inner
    }
}
impl<R: Read + Seek> ReadAt for SeekSource<R> {
    fn snapshot(&mut self) -> io::Result<Snapshot> {
        Ok(Snapshot {
            length: self.inner.seek(SeekFrom::End(0))?,
            revision: self.revision,
        })
    }
    fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        let actual = self.inner.seek(SeekFrom::Start(offset))?;
        if actual != offset {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "source did not seek to requested position",
            ));
        }
        self.inner.read(buffer)
    }
}

/// Half-open absolute byte extent, without any container validity claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteExtent {
    /// First byte.
    pub start: u64,
    /// Exclusive end.
    pub end: u64,
}

/// Limits for each explicit operation. Duration does not impose a total limit.
#[derive(Clone, Copy, Debug)]
pub struct ReadLimits {
    /// Maximum successful bytes returned by the provider in one operation.
    pub max_bytes: u64,
    /// Maximum provider read calls, including errors and Interrupted.
    pub max_calls: u64,
    /// Maximum logged extents. Zero disables logging, not exact counters.
    pub max_extents: usize,
}
impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            max_bytes: 16 * 1024 * 1024,
            max_calls: 65_536,
            max_extents: 1024,
        }
    }
}

/// Actual I/O in the current operation, including repeated reads.
#[derive(Debug)]
pub struct ReadReport {
    /// Valid successful byte counts returned, including bytes rejected by a
    /// later snapshot check. An invalid count is not added. Errors are required
    /// not to return partial progress.
    pub bytes: u64,
    /// Provider read calls, including Interrupted, WouldBlock and errors.
    pub calls: u64,
    /// Actual read extents in access order. Only adjacent reads are coalesced.
    /// These are not verified media intervals; skipped bytes are never included.
    pub extents: Vec<ByteExtent>,
    /// False if log storage filled or allocation failed. Counters remain exact;
    /// callers must not infer missing extents. A truncated log never resumes.
    pub extents_complete: bool,
}
impl Default for ReadReport {
    fn default() -> Self {
        Self {
            bytes: 0,
            calls: 0,
            extents: Vec::new(),
            extents_complete: true,
        }
    }
}

/// Positional I/O failure, kept separate from container parse failures.
#[derive(Debug)]
pub enum SourceError {
    /// Invalid limit configuration.
    Configuration,
    /// Snapshot changed. Permanently latched even if the old token reappears;
    /// construct a fresh reader to use a new snapshot.
    SourceChanged,
    /// Offset exceeds the complete snapshot. No provider read was attempted.
    OutOfBounds {
        /// Rejected absolute offset.
        offset: u64,
    },
    /// Final EOF before the advertised complete length. Permanently latched.
    UnexpectedEof {
        /// Offset where the provider returned zero.
        offset: u64,
    },
    /// Provider returned more bytes than requested. Permanently latched.
    InvalidRead {
        /// Absolute offset of the invalid read.
        offset: u64,
    },
    /// Work allowance exhausted. Begin a new explicit operation to continue.
    Limit,
    /// The provider could not satisfy this request now (WouldBlock). The extent
    /// is the capped request, not proof every byte in it is missing. Retry after
    /// making progress, within the remaining allowance or a new operation.
    NeedRange(ByteExtent),
    /// Provider I/O failure, including Interrupted. No automatic retry occurs;
    /// retries consume calls and must obey the caller's cancellation policy.
    Io {
        /// Absolute request offset, absent for a snapshot check.
        offset: Option<u64>,
        /// Original provider error.
        error: io::Error,
    },
}
impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration => f.write_str("nonzero read byte/call limits required"),
            Self::SourceChanged => f.write_str("source snapshot changed"),
            Self::OutOfBounds { offset } => write!(f, "source offset {offset} exceeds snapshot"),
            Self::UnexpectedEof { offset } => write!(f, "source ended before snapshot at {offset}"),
            Self::InvalidRead { offset } => write!(f, "invalid provider read count at {offset}"),
            Self::Limit => f.write_str("positional read work limit"),
            Self::NeedRange(extent) => write!(
                f,
                "source range {}..{} unavailable",
                extent.start, extent.end
            ),
            Self::Io { offset, error } => write!(f, "source I/O at {offset:?}: {error}"),
        }
    }
}
impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
enum Failure {
    Changed,
    Eof(u64),
    InvalidRead(u64),
}
impl Failure {
    fn error(self) -> SourceError {
        match self {
            Self::Changed => SourceError::SourceChanged,
            Self::Eof(offset) => SourceError::UnexpectedEof { offset },
            Self::InvalidRead(offset) => SourceError::InvalidRead { offset },
        }
    }
}

/// Own one immutable snapshot with explicit per-operation work budgets.
///
/// No checkpoint import or mutable access to the provider is exposed. A failed
/// read may have modified the caller's buffer; use bytes only after success.
/// Dropping a reader cancels no external request or blocking provider call.
pub struct RangeReader<S> {
    source: S,
    snapshot: Snapshot,
    limits: ReadLimits,
    report: ReadReport,
    failure: Option<Failure>,
}
impl<S: ReadAt> RangeReader<S> {
    /// Capture the snapshot and start the first bounded operation.
    pub fn new(mut source: S, limits: ReadLimits) -> Result<Self, SourceError> {
        if limits.max_bytes == 0 || limits.max_calls == 0 {
            return Err(SourceError::Configuration);
        }
        let snapshot = source.snapshot().map_err(|error| SourceError::Io {
            offset: None,
            error,
        })?;
        Ok(Self {
            source,
            snapshot,
            limits,
            report: ReadReport::default(),
            failure: None,
        })
    }
    /// Originally captured snapshot, never replaced by a later token.
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
    }
    /// Current operation's actual I/O. Snapshot checks are not read_at calls
    /// and do not count here; an adapter may perform extra seeks for them.
    pub fn report(&self) -> &ReadReport {
        &self.report
    }
    /// Check identity and explicitly reset the work budget/log. Does not reset
    /// terminal failures or establish a container validation boundary. On a
    /// failed check, the previous report and allowance remain unchanged.
    pub fn begin_operation(&mut self) -> Result<(), SourceError> {
        self.check_snapshot()?;
        self.report.bytes = 0;
        self.report.calls = 0;
        self.report.extents.clear();
        self.report.extents_complete = true;
        Ok(())
    }
    /// Check identity before publishing state derived from earlier reads.
    /// Provider snapshot I/O errors may be retried; identity changes latch.
    pub fn check_snapshot(&mut self) -> Result<(), SourceError> {
        if let Some(failure) = self.failure {
            return Err(failure.error());
        }
        let current = self.source.snapshot().map_err(|error| SourceError::Io {
            offset: None,
            error,
        })?;
        if current != self.snapshot {
            self.failure = Some(Failure::Changed);
            return Err(SourceError::SourceChanged);
        }
        Ok(())
    }
    /// Read a bounded prefix of a request. Checks snapshot identity before and
    /// after every provider read. At the advertised end or for an empty buffer,
    /// returns zero without reading or consuming allowance (after checking the
    /// snapshot and offset). Requests are capped to source end and byte budget.
    /// NeedRange reports that capped request exactly. No retry is automatic.
    pub fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, SourceError> {
        self.check_snapshot()?;
        if offset > self.snapshot.length {
            return Err(SourceError::OutOfBounds { offset });
        }
        if buffer.is_empty() || offset == self.snapshot.length {
            return Ok(0);
        }
        if self.report.calls >= self.limits.max_calls || self.report.bytes >= self.limits.max_bytes
        {
            return Err(SourceError::Limit);
        }
        let available =
            (self.snapshot.length - offset).min(self.limits.max_bytes - self.report.bytes);
        let count = buffer
            .len()
            .min(usize::try_from(available).unwrap_or(usize::MAX));
        // count <= snapshot.length - offset; neither endpoint can overflow.
        let requested = ByteExtent {
            start: offset,
            end: offset + count as u64,
        };
        self.report.calls += 1;
        let result = self.source.read_at(offset, &mut buffer[..count]);
        if let Ok(len) = result {
            if len > count {
                self.failure = Some(Failure::InvalidRead(offset));
                return Err(SourceError::InvalidRead { offset });
            }
            self.report.bytes += len as u64;
            self.record(offset, len);
        }
        // Reject a snapshot change during I/O, even on an error return.
        // Successful bytes still count as actual read work.
        self.check_snapshot()?;
        match result {
            Ok(0) => {
                self.failure = Some(Failure::Eof(offset));
                Err(SourceError::UnexpectedEof { offset })
            }
            Ok(len) => Ok(len),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                Err(SourceError::NeedRange(requested))
            }
            Err(error) => Err(SourceError::Io {
                offset: Some(offset),
                error,
            }),
        }
    }
    fn record(&mut self, offset: u64, len: usize) {
        if len == 0 || !self.report.extents_complete {
            return;
        }
        let end = offset + len as u64;
        if let Some(previous) = self
            .report
            .extents
            .last_mut()
            .filter(|extent| extent.end == offset)
        {
            previous.end = end;
        } else if self.report.extents.len() < self.limits.max_extents
            && self.report.extents.try_reserve(1).is_ok()
        {
            self.report.extents.push(ByteExtent { start: offset, end });
        } else {
            // Explicit telemetry omission. Never coalesce across an omitted
            // extent or imply that the retained prefix covers the operation.
            self.report.extents_complete = false;
        }
    }
    /// Recover the provider. Validation state must not be reused with another
    /// reader/snapshot merely because its reported length matches.
    pub fn into_inner(self) -> S {
        self.source
    }
}
