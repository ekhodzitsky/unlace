# Sequential audio demuxing

The unreleased `demux` module emits original encoded audio packets over `Read`,
without requiring `Seek`. It integrates the [streaming core](streaming.md) and
[block parser](blocks.md). This is an early, explicitly bounded subset, with no
production, universal-conformance or performance claim. Selected packet bytes
now pass the pinned [producer corpus checks](packet-qualification.md) against
FFmpeg and Nestegg. Broader timing, trim, hostile-input and performance
qualification remain unfinished. Ordinary builds/tests need no external tools
or media.

## Consumer contract

Construct `Demuxer::new(source, AudioSelection, DemuxLimits)` and call `poll`.
`First` selects the first enabled audio entry in file order; `Number` selects an
exact enabled audio TrackNumber, rather than a vector index. Missing, disabled
or nonaudio selection returns AudioTrackUnavailable. Every track remains
available through `tracks()`, including number/UID/type, codec ID/private data,
delay/preroll, DefaultDuration, MaxBlockAdditionID and audio settings. `TracksReady` indicates that
selection and track metadata are finalized; Info may still follow Tracks.

A Packet borrows the demuxer's current block storage until the next mutable
call. Copy its bytes only if retention is needed, and impose an application
limit on retained output. No packet bytes are decoded, trimmed, reordered or
transformed. Unselected blocks are structurally parsed and track-checked, then
omitted; their data is buffered under the same block limit. Large nonaudio
blocks can therefore exceed configured limits. This implementation makes no
zero-copy source-I/O claim: chunks are copied into one reusable block buffer,
then emitted as borrowed frame slices.

The private 0.2.1 implementation retains one initialized 8 KiB I/O scratch and
one 256-entry checked frame-range table per Demuxer, reusing them across polls and
blocks. Scratch previously lived temporarily in each advance call; its retained
per-instance cost is now explicit. This adds no heap allocation, changes no
public byte/work limit and does not retain recording history. No source/API
migration is required from 0.2.0.

A BlockGroup retains its block until the group ends, so ReferenceBlock,
BlockDuration and signed DiscardPadding appearing after Block cannot be lost.
Whole-block signed padding is available on every laced frame. A separate
once-per-block notification appears on the last frame for positive padding or
first for negative padding; padding can span more than that one frame. Bytes remain intact even when padding is
nonzero. Packet block offsets identify the original Block/SimpleBlock element;
all laced frames share the same offset and carry their index/count explicitly.

`NeedInput` preserves all state after WouldBlock. `Yield` bounds work and requires
another poll. Other errors are terminal and further polls return Failed. I/O
causes, low-level truncation/syntax errors, unsupported features and resource
refusals remain distinguishable. Zero from Read means final EOF, so temporarily
empty producers must return WouldBlock. No portable checkpoint or source-swap
operation is offered; `into_inner` discards continuation and lookahead.

## Timestamps and audio metadata

The supported TrackTimestampScale is exactly 1.0. A block's first raw timestamp
is `(Cluster Timestamp + signed relative timestamp) * TimestampScale` in signed
`i128` nanoseconds, using checked arithmetic. `block_timestamp_ns` preserves
this value before CodecDelay. `timestamp_ns` subtracts CodecDelay exactly once.
Negative timestamps are preserved. Consumers must not subtract that delay again.
SeekPreRoll is reported in nanoseconds and does not change emitted bytes/times.
No decoder preroll or sample-level trimming is performed by this layer.

For later laced frames, DefaultDuration allows an offset of
`lace_index * DefaultDuration` nanoseconds. Without it, `timestamp_ns` is None:
the frame timestamp is underdetermined by the block, not silently duplicated
from the first frame. The shared raw origin and lace order remain available to
a consumer with codec durations. BlockDuration is exposed for the whole block
in nanoseconds; it is not divided into invented per-frame durations. These
contracts follow [RFC 9559 sections 10 and 11](https://www.rfc-editor.org/rfc/rfc9559.html#section-10).
The [Opus timing contract](opus-timing.md) specifies header validation, exact
pre-skip, integer-delay quantization and located errors. Independent mono/stereo
start/end sample qualification now passes within its stated scope; broader
decoder/preroll coverage remains open.

## Supported and excluded structure

One finite EBML header and one Segment are supported, with `webm` or `matroska`
DocType, EBMLVersion/ReadVersion 1 and DocTypeReadVersion through 4. Version
relationships, declared ID/size maxima, required track fields, unique track
numbers/UIDs, scalar widths/ranges, relevant singleton occurrences and required
Info application fields are checked. Empty elements with declared defaults use
those defaults. Integer defaults and size syntax follow
[RFC 8794](https://www.rfc-editor.org/rfc/rfc8794.html).

Info and Tracks must precede Clusters; Cluster Timestamp must precede its blocks.
Other valid orders return Unsupported for this sequential contract. Segment and
Cluster may have unknown size. BlockGroup and metadata masters must be finite.
No/fixed/Xiph/EBML lacing, FlagLacing, reference presence, BlockDuration and signed
DiscardPadding are interpreted. Audio tracks require Audio settings. Nonunit
TrackTimestampScale, nonzero TrackOffset, ContentEncodings (including encryption
and compression), TrackOperation, codec state changes and addition mappings return
Unsupported, including when encountered on an unselected track. BlockAdditions
for selected audio also return Unsupported before packet emission. Finite
BlockAdditions on unselected tracks are skipped opaquely after resolving the
group track, even when additions precede Block. Outer boundaries and skip limits
are checked; their descendants and semantic validity are not. MaxBlockAdditionID
is retained as a declaration and does not itself imply additions occur. CRC-32 returns
Unsupported until checksum verification is implemented; it is never ignored.
Chained EBML documents and multiple Segments return Unsupported.

Known finite SeekHead, Cues, Tags, Chapters, Attachments, Video and selected
ancillary scalar values are skipped opaquely. Their outer boundaries and skip
size limits are checked; their descendants, checksums, index destinations and
semantic validity are not. Unknown IDs fail explicitly. The static schema in
`src/demux/schema.rs` is the exact recognized set. Not every Matroska extension,
scalar constraint, version-dependent rule or codec-specific WebM constraint is
validated. End therefore reports this subset's structural result, not universal
Matroska/WebM conformance or source authenticity. The separate
[seek reader](seeking.md) adds bounded index/destination checks.

## Completion and resource limits

Continue polling after the last packet. Only End confirms final source EOF and
the required supported document structure. FiniteSegment means a declared finite
Segment boundary was also reached. UnknownSizeSegment preserves uncertainty:
EOF might be a clean live stop or missing intended tail packets. Even a finite
boundary cannot prove what a producer intended to write. Duration is advisory,
not a proof of packet or decoded-audio completeness. A late malformed field,
truncated block/group or source failure returns an error after earlier packets.

Defaults retain at most 64 tracks and 4 MiB combined declared payload bytes for
EBML, Info and Tracks, counting nested data once. Each retained metadata leaf
also obeys framing metadata limits. One block is retained, bounded by block and
framing payload limits. All allocations use fallible reservation. Retained
vector capacity can exceed current lengths through allocator growth; it remains
bounded by the configured sizes/counts, rather than recording duration. The
framing stack is fixed at 64 slots and a packet table at 256 frame ranges.
No whole recording, packet history or index is accumulated.

Each poll handles at most `max_events_per_poll` framing events (default 64),
each with independent framing read-call/byte limits. One packet is emitted per
poll. A malicious stream of ignored elements still yields within those budgets.
Block parsing separately bounds lacing work. Counters for metadata count only
the initial EBML/Info/Tracks; long sequences of Clusters do not consume a lifetime
metadata quota. Source implementation latency/work inside Read remains outside
these budgets.

Run `cargo run --offline --locked --example audio_packets -- INPUT [TRACK_NUMBER]`
for a sequential file example. It emits JSON Lines with track metadata, packet
hex/timestamps/padding and a final completion event. Failure exits nonzero and
emits no successful final event. This verbose correctness manifest is not a
throughput benchmark.

## Continuation example

Run `cargo run --offline --locked --example continue_audio -- INPUT.webm [TRACK_NUMBER]`
to simulate a producer pause after each read of at most 17 bytes. The source
returns WouldBlock while paused and zero only at final EOF. The consumer resumes
the same owned demuxer after NeedInput, handles Yield separately, processes each
borrowed packet before the next poll and requires End. The example retains no
packet result history; its counters do not substitute for the reported completion
kind. Real applications wait for producer availability outside the library.
