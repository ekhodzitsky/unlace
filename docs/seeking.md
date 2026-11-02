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

Eighteen original synthetic seek tests check exact forward/backward packet
suffixes, selected-track identity, raw/effective timing, fixed laces and final
padding, missing/video-only cues, group/no-relative hints, every track VINT width,
forged pointers into matching payload bytes, incorrect times/tracks/relative
positions, SeekHead bounds/overflow, finite truncations at every byte, every-byte
range holes, short/interrupted reads, preparation and packet cancellation, source
replacement, unsupported CRC classification and explicit resource/work limits.
Four defects were reproduced before correction: eight-byte VINT mask overflow,
request-overflow poisoning, treating unsupported index CRC as corrupt input, and
misclassifying a missing Cluster timestamp as no available access point.

Local corpus qualification uses the eight original/pinned sources in
[packet qualification](packet-qualification.md) and [Opus timing](opus-timing.md).
The local adapter compares every suffix packet byte and all packet fields with
full sequential iteration on the identical file and selected track. It uses a
fresh sequential file reader per query and repeats forward/backward requests on
one seek reader. It also compares each requested one-second timestamp interval against full
iteration (including packets before the skipped boundary) and requires both
paths to reach their final completion. All corpus packet timestamps in this
check are determinate.
Baseline ordered packet bytes on these producer cases were independently checked
as documented in those reports; this seek check is same-library suffix
comparison, not a new independent decoder or competitor seek comparison.

Four queries per source passed (32 total). The seven short sources used
0, 300, 100 and 2000 milliseconds, in that order. Big Buck Bunny used
0, 300, 20 and 590 seconds. The second query's provider reads are below; opening
is shown separately and is paid once per reader. Repeated reads count repeatedly.

| Source | Open bytes | Seek bytes | Matched suffix packets | Full selected packets | Second-query path |
| --- | ---: | ---: | ---: | ---: | --- |
| Browser Opus | 488 | 108 | 25 | 28 | Missing Cues scan |
| Browser VP9/Opus | 589 | 120 | 26 | 28 | Missing Cues scan |
| Synthetic Opus | 483 | 373 | 52 | 52 | Selected cue |
| Synthetic Vorbis live | 3722 | 9 | 21 | 21 | Missing Cues scan |
| Synthetic VP9/Opus | 552 | 10 | 21 | 21 | No selected-track cue scan |
| Opus mono trim source | 483 | 373 | 52 | 52 | Selected cue |
| Opus stereo trim source | 484 | 373 | 52 | 52 | Selected cue |
| YouTube Big Buck Bunny | 1781 | 3505 | 15325 | 29825 | Selected cue |

The 300-second Big Buck Bunny query starts at Cluster byte 4,785,185 and actual
selected time 289,994,500,000 ns, with declared preroll 80,000,000 ns. Reading its
suffix used 4,945,353 provider bytes; full iteration used 9,730,538. These are
actual direct `File` read counts in a local correctness adapter, not disk/network
traffic, cold/warm cache measurements, latency, allocations or throughput. Source
checks perform additional seeks. No numerical performance ranking is claimed.

Checks ran on Linux 7.0.0-38-generic x86_64, Rust 1.89.0
(`29483883eed69d5fb4db01964cdf2af4d86e9cb2`, LLVM 20.1.7), debug builds with
standard limits, no external runtime/dependency in ordinary tests. Source hashes,
rights and producer versions are in the linked reports. Commands used the local
`runs/seek-qualification/check` adapter, for example:

```sh
runs/seek-qualification/check runs/demux-qualification/youtube-bbb.webm \
  0 300000000000 20000000000 590000000000
```

Raw adapters, sources and JSONL evidence remain local under ignored `runs/`.
Sustained fuzzing, multi-hour hostile/index coverage, additional platforms,
matched competitor seeks and performance qualification remain separate work.

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
