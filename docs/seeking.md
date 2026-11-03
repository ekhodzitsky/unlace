# Container seeking

`seek::SeekReader<S>` adds bounded selected-audio access over `source::ReadAt`.
It validates the same metadata and emits the same borrowed packet fields as the
sequential demuxer. It does not decode, reconstruct codec state, clip packets to a
time interval, or verify media skipped before the chosen Cluster. See the
[source contract](sources.md) for immutable revisions, external range caches,
provider read accounting and the limitations of generic `Read + Seek` inputs.

Run the complete file example with signed presentation nanoseconds:

```sh
cargo run --offline --locked --example seek_audio -- INPUT.webm 300000000000
cargo run --offline --locked --example seek_audio -- INPUT.webm 300000000000 2
```

The example owns a `File` through `SeekSource`; the caller must keep its bytes
immutable. It prints preparation reads separately from seek reads, the actual
access point, packet offsets/timestamps/trim and final suffix completion. This is
a correctness example, not a throughput benchmark. It flushes output explicitly.

## Opening and validation

Construction captures a complete source length and revision without reading
payloads. Call `prepare(&cancel)` until `Preparation::Ready`. `Yield` preserves
completed work. Unavailable ranges are located `Source(NeedRange)` errors; fetch
outside unlace and retry. Metadata preparation stops before consuming the first
packet. The selected enabled audio track is determined by `AudioSelection`, not
by whatever track appears in Cues.

Opening walks Segment children from the actual Segment payload start. A finite
child's checked declared length leads to the next header; compressed payloads
are skipped without searching for ID byte patterns. Unknown-sized Clusters end
at schema-valid Segment siblings or the Segment/source boundary, with their
finite children walked by checked lengths. Only Segment and Cluster may have
unknown size in this subset. This walk attests structural boundaries, not packet
contents. Finite Clusters do not require opening every block.

The bounded catalog is then used to parse finite SeekHead/Cues payloads. Every
SeekHead target must be an attested Segment child with the exact SeekID. Every
CueClusterPosition must identify an attested Cluster. Every CueTrack must exist
in validated Tracks. Offsets are relative to the Segment payload; optional
CueRelativePosition is relative to the Cluster payload. Arithmetic and source
bounds are checked before use. No untrusted SeekHead chain is followed, so loops
or self-references cannot create recursive work. These semantics follow
[RFC 9559](https://www.rfc-editor.org/rfc/rfc9559.html#section-5.1.5).

CueTime and CueTrackPositions can occur in either order. Duplicate singleton
fields, missing required fields, zero track/block numbers, invalid integer
widths, unknown-sized index elements and child overruns are errors. Void is
permitted. CRC verification, CueReference/extensions and selected nonzero
CueCodecState are explicitly unsupported. CueDuration and nonzero CueBlockNumber
are syntactically checked hints, never used to position the reader; their values
are not claimed to be independently verified. Sequential `Demuxer<Read>` still
skips finite Cues/SeekHead opaquely and has no index-validation guarantee.

## Seeking and packet ownership

A `SeekRequest` uses the same signed nanosecond presentation timeline as packets,
including CodecDelay subtraction exactly once. The effective preroll is the
maximum of caller-requested preroll and raw Track.SeekPreRoll. Unlace adds no
codec recommendation silently. Overflow in the requested target returns
`RequestOverflow` without poisoning the reader.

For selected-track cues, choose the latest CueTime at or before the preroll
target, comparing exact checked nanoseconds. Cues need not be sorted. Before
using the chosen entry, walk the actual Cluster's child boundaries and block
prefixes to verify its selected track, time and any relative destination. A
relative position into payload bytes or onto another track is rejected. A
BlockGroup destination is the group's header, and its contained Block is checked.
Lacing/payload and late group timing checks remain the sequential parser's job
when actual packets are read; prefix inspection alone does not validate them.

Missing Cues, no selected-track cues and a target before the first selected cue
have distinct `Fallback` reasons. Fallback scans actual Cluster/block prefixes
under explicit work bounds, choosing the latest first-block timestamp before the
target, or the earliest available first block if none precedes it. It never
silently substitutes another track. Invalid used cues are errors, not a reason
to quietly fall back. The sequential parser restarts at the containing Cluster,
so it reads all selected frames there, including those before a cued BlockGroup.

`SeekResult` reports the request, actual first selected block time/offset, Cluster
boundary, fixed track, declared/effective preroll, optional cue time, fallback and
skipped prefix. `starts_before_preroll_target` is an exact timestamp comparison,
not evidence that decoder state or the required audio history is available. A
request near the beginning may have insufficient preceding history. The caller
must initialize/reset its decoder, process preroll and interpret all trim data.
No packet bytes or padding are consumed by seeking itself.

After a successful seek, call `poll(&cancel)` through `SeekEvent::End`. Packets
borrow the reader until its next mutable call, just as in sequential demuxing.
`Yield` retains work; `Source(NeedRange)` retains the exact core continuation.
End contains `SuffixCompletion`, explicitly scoped from the last committed
Cluster, including finite/unknown Segment completion evidence. It is never full
file or requested-time-interval completeness. Nonmonotonic timestamps and packets
before the skipped boundary may affect time queries; no chronological interval
coverage is inferred from a physical suffix. Producer completeness for unknown
sizes and decoder readiness remain uncertain.

## Limits, cancellation and errors

`SeekLimits` bounds retained Segment children and cue entries independently,
aggregate index payload bytes, parsed index elements, track positions per
CuePoint, preparation steps per call and headers scanned per seek. Fixed-depth
index parsing does not recurse into arbitrary input. `ReadLimits` independently
bounds provider reads/bytes/logged extents in each prepare/seek/packet poll call.
Counters include retries and repeated reads; opening/indexing cost is visible.
These are explicit storage/work refusals, not total recording-duration limits.
The sequential interface remains available without a growing catalog.

Completed boundary/index steps persist between preparation calls. A partial
header or individual finite index payload may be retried transactionally from
its beginning; cached source bytes must remain available. Allowances must be
large enough to finish one such unit, including short-read calls. An overly small
allowance can repeatedly refuse that unit; construct a reader with suitable
explicit limits rather than expecting an unbounded internal retry. No file-wide
payload/result buffer is allocated. Index allocations use fallible reservation.

Cancellation is cooperative via `AtomicBool`: it is checked between bounded
metadata/packet polls, between index reads/elements and immediately before
committing a seek or publishing a packet. It cannot interrupt a blocking provider
method or allocation. A cancelled/limited/unavailable seek leaves the old active
packet cursor and buffered packets intact; positional inspection reads still
count as I/O. Cancelling a packet poll may leave additional parsed data buffered,
but no packet is consumed before publication.

Invalid or unsupported index/container errors latch reader failure. Fatal source
errors routed through the sequential core also make that core terminal; the
original source error is returned before subsequent `Failed` responses. An
observed source revision/length change remains permanently latched by the source
layer. Only cancellation, unavailable ranges and work refusal are guaranteed
resumable; a limit may require a new configuration. No recovery scans cross
arbitrary corruption. There are no imported checkpoints or mutable source handles.

## Qualification

The [original seek campaign](history/seek-qualification.md) preserves all 32
corpus suffix/window comparisons, source/read counts and the four reproduced
regressions. Later [0.2.2](seek-performance-0.2.2.md), [0.2.3](seek-cache-0.2.3.md)
and [0.2.4](prepared-sessions-0.2.4.md) reports add source-pinned correctness,
performance and lifecycle evidence. The [concurrent consumer pilot](consumer-pilot-0.2.4.md)
qualifies a specific prepared local profile. These results do not establish
decoder readiness, arbitrary-provider latency or skipped-media validation.

## Grouped index reads in 0.2.2

Index preparation and cluster fallback group the minimum bytes needed by a valid
EBML header, and the first four bytes required by a valid Block prefix. Header
requests stop within the parent and never include a valid element's payload.
Successful short reads are inspected before requesting further bytes. Structural,
track, timestamp, source revision, cancellation and completeness checks remain in
force. No default, explicit budget or index storage limit increases.

Providers must support bounded multi-byte requests or return partial progress and
explicit availability errors. Request sizes and the order of source errors versus
malformed-input errors are not stable API guarantees: a provider requiring the
whole requested range can report `NeedRange` before an already available invalid
marker is inspected. Retrying after availability resumes still rejects that marker.
Call/byte reports describe the actual provider requests, so their counts depend
on provider short reads and cache boundaries.

There is no new cached cluster-time assumption. A fallback still verifies the
catalogued clusters on each seek, including repeated or backward requests. The
[0.2.2 qualification](seek-performance-0.2.2.md) separates reduced logical calls
from backing-cache traffic and measures first and repeated requests.


## Verified fallback reuse in 0.2.3

After a complete fallback scan and successful public seek commit, the catalogue
retains the first selected-track block offset/raw ticks or verified absence for
each Cluster. Repeated fallbacks inspect these summaries in original order and
charge their original header-work counts against the same max_scan_headers cap.
No chronological-order assumption or request-dependent timestamp is cached.
Chosen Cues still require independent track/time/relative-position attestation.

The ready cache belongs to the selected track and the reader's immutable source
snapshot. Every operation retains its source/cancellation checks. Revision changes
remain latched; correct provider revisions and immutability are required, not
cryptographically established. A partial scan or failed cursor transaction does
not publish staged summaries. Cached summary lookup performs no provider reads;
ReadReport therefore reports zero calls/bytes for that lookup. Snapshot methods
and subsequent suffix packet reads still perform their normal provider work.

Storage is bounded by the existing catalogue max_entries, without a second cache
allocation. Each entry includes summary fields even on chosen-cue-only readers.
On the measured x86_64 target this adds 64 bytes per entry, or up to 4 MiB nominal
extra entry storage at the default limit, plus capacity/allocator overhead.
Larger existing preparation allocations can fail earlier. max_index_bytes still
bounds finite Cues/SeekHead payloads separately. First preparation and fallback
read work are unchanged. The [0.2.3 report](seek-cache-0.2.3.md) retains memory,
first-use timing and failed criteria alongside reuse measurements. The earlier
0.2.2 profile above is historical; its every-seek re-scan description no longer
applies once a 0.2.3 fallback cache is committed.


## Prepared-source sessions in 0.2.4

`PreparedSource<S>` owns the original provider and ordinary reader state. Call
`prepare` until Ready: metadata/catalogue readiness always yields before a
separate complete selected-track fallback warmup operation. This preserves one
configured read budget per public call and makes all preparation cost visible.
Ready is stronger than `SeekReader::prepare`; it can return full-scan limits,
NoAccessPoint or prefix errors even when metadata-only preparation succeeds.
Chosen-cue-only sources also pay for the full warmup. No partial fallback cache
becomes usable after failed preparation.

`open(request, cancel)` returns an exclusive positioned `SeekSession` only after
successful seek and final source/cancellation checks. `start()` exposes the
actual access point/preroll, while metadata/read-report/poll methods retain the
ordinary reader's contracts. A failed opening returns no session and cannot
expose the previous consumer's pending laces. Session Drop performs no I/O and
never resets fatal or source-change latches. Compile-fail doctests enforce
exclusive borrowing and consuming source recovery. There is no shared cursor
or implicit concurrent-consumer support.

The original source handle remains alive. Equal snapshot tuples from independent
providers never attach another source to this verification. The API supplies no
serialized index, new-provider import, physical file reopening or process-restart
persistence. Provider honesty/immutable revisions remain required; ownership is
provenance within that contract, not cryptographic authentication. Application
registries own source-generation selection, instance counts and eviction policy.
`into_inner` consumes the owner and discards all verification.

The complete [example](../examples/prepared_audio.rs) reports preparation and
individual openings separately with unchanged default limits. The
[0.2.4 evaluation](prepared-sessions-0.2.4.md) compares fresh and already-retained
0.2.3 readers, varied previously unconsumed windows, advisory cache resets,
source identity/failure controls and the unknown-file first-use cost. A prepared
profile must explicitly adopt preparation outside its request deadline; the
original 250 ms new-file observations still fail and remain visible.
