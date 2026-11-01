# Positional source contract

`source::RangeReader<S>` bounds positional input over `source::ReadAt`. It owns
one snapshot and has no container interpretation. The sequential demuxer still
uses ordinary `Read` and does not require a known length or seeking. The separate
[seek reader](seeking.md) adds container index/destination checks.
Read extents alone never establish verified interval completion.

A range provider supplies a complete `u64` length and a `u128` revision through
`ReadAt::snapshot`. Bytes must remain immutable within that revision, including
same-length replacements. A revision must not be reused for different bytes.
Availability in an external cache may grow while length and revision remain
unchanged. An unknown-length live input should use the sequential interface.
No networking, range fetching, hashing or source authentication happens here.

For owned `Read + Seek` input, `SeekSource::new(cursor, revision)` performs
absolute reads and obtains length by seeking to the end. The application must
keep the underlying bytes immutable for its lifetime. Generic `Read + Seek`
cannot detect same-length external overwrites. Snapshot checks may move the
underlying cursor; a recovered cursor has its actual last I/O position. Use a
version-aware provider for externally replaceable content. Revision tokens are
only as reliable as that provider's contract, and changes that occur and revert
entirely between checks cannot be detected by observation.

`RangeReader::new` captures the token. Each read checks it before and after the
provider call. `check_snapshot` checks again before publishing a result derived
from earlier reads. An observed revision or length mismatch permanently latches
`SourceChanged`, even if the original token later returns. `begin_operation`
cannot clear it. There is no serialized/imported continuation or mutable provider
access; `into_inner` consumes the reader. Do not transfer validation state to a
new reader merely because its length matches.

`read_at(offset, buffer)` returns a successful byte count, including short reads.
The request is capped to the advertised end and the remaining byte allowance.
An empty request or a request exactly at the source end returns zero without a
provider read, after checking identity and bounds. A larger offset is rejected.
A nonempty provider read returning zero before the advertised end is a latched
`UnexpectedEof`; returning more than requested is a latched `InvalidRead`.
Neither is treated as temporary unavailability or successful container completion.

A provider's `WouldBlock` becomes `NeedRange(ByteExtent)`: this is the exact
capped request it could not satisfy, not proof that every byte in the extent is
missing. The caller obtains data outside unlace and explicitly retries. A
provider can return an available prefix first. `Interrupted` and other provider
errors remain located `Io` errors with the original cause. There is no automatic
retry loop. Snapshot failures carry no byte offset and can be retried, provided
no terminal failure was latched. If the post-read snapshot check fails, the
read returns an error even though its bytes count as actual I/O. A failed read
may have changed the buffer; consumers must only use a successful byte count.
Providers must return partial progress as a byte count before returning an error
on a later call; error returns cannot report partially delivered bytes.

`ReadLimits` independently caps successful bytes, provider read calls and
retained extent entries per explicit operation. Byte/call limits must be nonzero;
zero extent entries disables the log. Calls include errors, `Interrupted` and
`WouldBlock`, so repeated no-progress reads exhaust the allowance. Limit refusal
happens before another provider read. It does not establish a validation boundary
or poison the immutable source. `begin_operation` checks identity and explicitly
resets the counters/log, retaining allocated log capacity. A failed reset leaves
the old report and budget intact. Consumers must define bounded operations and
check cancellation between calls; resetting allowances in an unbounded internal
retry loop would defeat the work contract. There is no duration-dependent total
source size limit or automatic reset.

`ReadReport` records valid successful byte counts, calls and actual read extents
in access order, including repeated reads and bytes rejected by a post-read
check. Adjacent reads coalesce; overlapping or backward reads remain distinct.
Extents never include skipped bytes. If the bounded log fills or allocation
fails, `extents_complete` becomes false and logging stops for that operation.
The byte/call counters remain exact for a conforming provider. Do not infer
omitted extents or treat the retained log as complete. Snapshot calls are not
read calls; a `SeekSource` snapshot performs an additional end seek. These
counters therefore measure provider reads, not all underlying system calls,
filesystem traffic, network traffic or validated packet coverage.

All operations are synchronous. There are no background workers to cancel or
join. This layer never interrupts a blocking provider method. Applications that
need bounded cancellation latency must bound every provider method, including
snapshot checks, and check cancellation between calls. The container
[seek contract](seeking.md) adds cooperative cancellation and
validation state on top of this source layer.

The module's executable Rust example demonstrates backward reads and read
accounting. Original deterministic tests cover range continuation at every byte
prefix, short/interrupted reads, explicit retry, per-operation budgets, truncated
logs, observed A-B-A token changes, changes during I/O, post-read snapshot errors,
premature EOF, invalid provider counts, empty inputs, `u64::MAX` endpoints and
incorrect seek positions. These source tests do not qualify container seeking,
codec state restoration, network implementations or atomic external snapshots.
