# Encoded WebM writing

The `write` module writes already encoded Opus/Vorbis audio and
VP8/VP9 video. It performs no encoding, decoding, resampling, sample trimming or
codec random-access detection. It is a container writer with an explicit subset,
not a general Matroska remuxer. The
[writer qualification](writer-qualification.md) records independent output checks
and their limits. No writer performance or universal compatibility claim is made.

## Ownership and API

Construct `LiveWriter<W: Write>` or `IndexedWriter<W: Write + Seek>` with borrowed
`TrackConfig` entries and `WriteConfig`. Construction consumes the configuration
only long enough to validate and serialize it; the writer retains compact track
state, not borrowed configuration. `TrackConfig::from_audio` borrows the audio
metadata exposed by the demuxer and explicitly rejects declared additions.
Video dimensions must be provided separately because the selected-audio demuxer
does not retain them. Unknown source fields are not claimed to be preserved.

`EncodedBlock::new(track, presentation_ns, bytes)` describes a caller-declared
random-access single frame. Set `Frames::Laced` with two through 256 slices and
Fixed, Xiph or Ebml lacing to retain a whole block. None lacing and single-frame
laced representations are rejected. Original bytes, frame order/boundaries,
track numbers/UIDs, CodecPrivate, enabled/lacing flags, supported audio/video
settings, DefaultDuration, CodecDelay and SeekPreRoll remain unchanged. The
writer borrows frame slices only during the call. Normal blocks are copied into
one bounded pending Cluster; an oversized valid block is sent directly to a
standalone Cluster without retaining its payload.

Blocks remain in caller order. Defaults pack finite Clusters with at most 1 MiB
of pending payload and less than one second of raw block-origin span. The signed
16-bit relative timestamp range can close a Cluster sooner. Size, time and tick
bounds never split a block or change lacing. A valid block larger than the buffer
gets its own direct-write Cluster; it does not force a larger retained buffer.
Set `max_cluster_duration_ns` or `max_cluster_buffer` to zero to disable packing.

`write_block` accepts data but may not yet emit it. `buffered_bytes()` reports
retained payload length; capacity and allocator overhead are additional. After
an I/O failure some retained bytes may already have reached the sink, so this is
not a resumable count. `position()` is the confirmed sink cursor, not a planned
end including buffered media. Explicit `flush()` emits the pending finite Cluster
and flushes the sink while leaving the writer in Writing. Use it to control live
latency; the media-time span is not a wall-clock timer or background task.
The writer never reorders or discards accepted blocks, except that explicit
extraction/Drop aborts and discards any pending data without a finish receipt.

Call `finish` explicitly, check its result, and retain `WriteReceipt` when a
consumer needs evidence of successful creation. `into_inner` and Drop never
finish or repair an output. A buffered sink may perform its own destructor flush;
that is not writer finalization. Passing a mutable sink borrow permits the caller to
inspect/recover a partially written sink even when construction fails. There is
no mutable sink accessor that bypasses the writer's cursor/state bookkeeping.

## Exact clocks, lacing and trim

`EncodedBlock.timestamp_ns` is the first frame presentation timestamp in signed
nanoseconds, after CodecDelay. The writer adds the configured delay once to
recover the raw block clock. The raw clock and optional whole-block duration
must be exactly representable at `timestamp_scale_ns`; an inexact value is an
input error before block output. No timestamp is silently rounded. DefaultDuration
is already in nanoseconds and is not rescaled or inferred from codec packets.

Raw block clocks must be nondecreasing globally, allowing equal clocks across
tracks. Each track's known frame timestamps must also be nondecreasing, including
the last laced frame when DefaultDuration makes its time known. Unknown later
lace timestamps remain unknown. A negative raw clock uses Cluster Timestamp zero
and a signed relative timestamp, down to -32,768 ticks; other Cluster timestamps
fit u64. All presentation/delay arithmetic is checked in i128. A caller chooses
a finer scale when source clocks need it. The example preserves an already
available source scale and uses exact one-nanosecond ticks if Info follows Tracks.
Fine ticks can force more Clusters; clock precision is never traded for packing.

Whole-block signed DiscardPadding is written as metadata once, without changing
encoded bytes. Positive means discard from the block end; negative from its
beginning, including padding spanning laced frames. Padding exceeding an explicit
BlockDuration or a duration derived from DefaultDuration is rejected. Without a
known duration, the writer preserves the raw value and makes no decoded-duration
claim. OpusHead uses the same channel/input-rate and exact pre-skip/adjacent-integer
CodecDelay policy documented in [opus-timing.md](opus-timing.md). Pre-skip and
raw delay are never rewritten or applied to packet samples.

Keyframe, invisible and SimpleBlock discardable flags are caller declarations.
A BlockGroup is used for padding, explicit duration or references. Discardable
cannot be represented on that group and is rejected. A non-key BlockGroup with
no supplied references uses ReferenceBlock zero, explicitly meaning unknown
reference, following [RFC 9559 section 10.1](https://www.rfc-editor.org/rfc/rfc9559.html#section-10.1).
Supplied references are signed Track Ticks with TrackTimestampScale one. They
are not fabricated from packet contents; reference zero cannot be combined with
other references. A keyframe with references is inconsistent and rejected.

## Live and indexed completion

Live mode requires a fresh exclusive sink at its logical start. It emits an
unknown-size Segment and finite Clusters, without Cues or retained index history.
`finish` emits pending media, flushes and returns `OutputCompletion::Live`. There is no on-wire marker
proving that a live receiver obtained every intended packet. Both stopping without
finish and a completed live file can therefore demux as UnknownSizeSegment.

Indexed construction verifies an empty sink positioned at zero. During writing
the Segment size remains unknown. Eligible nonnegative caller-declared keyframes
are indexed for each enabled track at the configured minimum cue interval.
Both audio and video tracks can have cues; the writer favors selected-track
access rather than the WebM guideline's smaller video-only index. There is no
claim that an index proves decoder readiness. Reaching a cue count/byte limit
refuses the next indexed block before writing it; entries are never dropped
silently. At least one eligible cue is required to finish indexed output.

Indexed finish first emits pending media, then appends finite Cues, patches a reserved front SeekHead for Info,
Tracks and Cues, flushes those writes, patches the eight-byte finite Segment
length, restores the end position and flushes again. Only success returns
`OutputCompletion::Indexed`. It records separate packet/block/Cluster/cue counts and the end
position. No Duration is invented from the last timestamp when encoded duration
is unknown. Both modes reject finishing an empty recording.

## Errors, publication and durability

Preflight validation, allocation and index-cap refusals leave Writing and emit
none of the rejected block. An I/O error, zero/invalid write count, exhausted I/O
call budget or incorrect seek position is terminal. State becomes Failed;
subsequent writes/finish return State without retrying partial output. WouldBlock
is also terminal for this synchronous writer. Interrupted calls retry within the
explicit work allowance. A caller requiring resumable nonblocking output must
provide its own bounded queue/transport; the writer has no portable checkpoint.

WriteError retains typed OpusHead causes, including unsupported versions/mappings,
and the source I/O error, operation stage and last confirmed
logical cursor. Buffered sinks may still have unflushed bytes. The sink must be
exclusively owned and implement Write/Seek honestly; a successful returned position
does not authenticate externally overwritten bytes. Errors do not roll back
writes or guarantee that a failing provider made no external changes.

A generic Write + Seek sink cannot guarantee atomic publication or storage
durability. In particular, a final flush can fail after a finite Segment header
is already visible. A regression verifies that such bytes can parse as finite,
while the writer returns an error, latches Failed and produces no receipt. Do not
use parseability or a destructor as proof of successful creation. When external
observers must see only successfully created files, publish a staged file only
after checked finish and the application's durability operation.

The example uses a same-directory `OUTPUT.unlace-partial` created with
`File::create_new`, checks input End, writer finish, buffered sink extraction and
`File::sync_all`, then publishes with a hard link that cannot replace an existing
destination. Failed input/finalization leaves staging data and no newly published
file. Staging cleanup failure after publication is reported separately. Existing
staging/destination files are never overwritten. Filesystems without hard-link
support report an error; there is no silent unsafe fallback. Directory durability
and platform-specific publication policies remain application responsibilities.

## Limits and supported metadata

Defaults allow 64 tracks, 4 MiB serialized initial metadata, 16 MiB block/frame
payloads, 1 MiB pending Cluster payload, 256 frames, a 64 KiB lace header, 65,536
cues, 8 MiB serialized Cues and
1,000,000 sink calls per operation. Track count and aggregate variable metadata
are checked before scanning codec initialization. Serialization uses fallible
reservation. Retained vector capacity can exceed lengths through standard
allocation growth; it is bounded by configured payload/count limits rather than
recording duration. One header buffer is reused, with separate small envelope and
BlockGroup suffix buffers, plus one bounded pending Cluster. No full media
recording or unbounded packet history is retained. Caller-selected sinks such as Vec can themselves retain all output.

Live mode retains no cues. Indexed mode retains four u64 values per cue plus
vector capacity/allocator overhead. Caller-selected cue intervals control index
density; an explicit limit refusal remains possible on a long or densely indexed
recording. Frame/lace/reference loops and metadata work are bounded independently;
finish work is bounded by retained cue count and the I/O-call allowance. Sink
implementation latency and allocator internals are outside those work counters.
Indexed output obeys the format's finite Segment size range. Live unknown-size
Segments have no artificial finite-length encoding cap; checked u64 cursor/count
bounds still apply. Arithmetic boundary tests do not claim petabyte-scale I/O.

Opus initialization is validated. Vorbis private data is checked for the three
Xiph-delimited header signatures, not decoded or fully verified as codec setup.
VP8 requires empty CodecPrivate; VP9 initialization is retained opaquely. Audio
rates must be positive/finite, channels/depth and video dimensions nonzero.
Codec bytes do not establish that caller-supplied payloads are decodable.

Only the documented fields are writable. Encryption, content compression, codec
state changes, block additions, attachments, chapters, tags, arbitrary codecs,
nonunit TrackTimestampScale and general Matroska extensions are outside this API.
The writer emits WebM version four/read-version two, as the pinned libwebm muxer
does for its supported elements; no universal WebM guideline or Matroska conformance
claim follows. Source-specific metadata absent from the configuration is not
claimed to be preserved by a general remux operation.

## Example and version impact

```sh
cargo run --offline --locked --example remux_audio -- INPUT.webm OUTPUT.webm indexed
cargo run --offline --locked --example remux_audio -- INPUT.webm OUTPUT.webm live 2
```

This example deliberately extracts selected audio, preserving its supported
metadata and packet/trim semantics. Other tracks are omitted. Laces are copied
under one block-size cap so group metadata is kept together; output uses EBML
lacing for those frames. That changes the length representation, not frame
boundaries. A finalized output from an unknown-size input proves completeness
of the observed output bytes, not the original producer's intended recording.
The example reports both input and output completion separately.

The initial writing API and packing update are included in the private 0.2.0
source milestone.
Relative to the first writer source, `WriteConfig` literals need
`max_cluster_duration_ns`, `WriteLimits` literals need `max_cluster_buffer`, and
`WriteReceipt` literals/patterns must account for `clusters`. `blocks` counts
encoded blocks, which no longer imply the same Cluster count. Default
`write_block` now buffers one bounded Cluster: call `flush` or `finish` before
observing emitted media; `into_inner` and Drop discard pending bytes. Setting
either packing bound to zero restores per-block direct emission. This is an
explicit incompatible source/API behavior change included in 0.2.0. Cargo and
the changelog identify that private source milestone; no release/tag/publication
is implied.
The crate's [SemVer policy](versioning.md) governs future compatibility. There is
no separately versioned serialized writer continuation or checkpoint format.
