# Accepted product direction

Accepted: 2026-11-01. The owner selected the name unlace and private development,
with the ambition to improve on the strongest existing container libraries.

Unlace owns container parsing and writing. Codec synthesis, resampling, network
access, download identity, transcription and agent response JSON remain consumer
responsibilities. A demuxer emits encoded packets and the metadata needed to
interpret them; it does not promise decoded audio simply because packets parsed.

## Design contracts

- Pure Rust, no unsafe and no third-party Cargo dependencies in any category.
- Sequential input must not require Seek. Seeking is an additional capability
  for seekable or externally ranged readers, with observable I/O requirements.
- Distinguish incomplete input, clean completion, unsupported features, invalid
  data, resource refusal and I/O failure. Never turn a truncated source into
  complete success or recover silently across arbitrary corruption.
- Bound payload, metadata, index, nesting and recovery work independently of
  the recording's duration. A declared length is not an allocation instruction.
- Preserve encoded packet bytes, track identity, signed timing and trim metadata.
  Parsing a block must not double-apply Opus delay or discard the final packet.
- Handle valid unknown-sized elements using schema and parent boundaries, not
  a byte-pattern search through compressed data.
- Validate index destinations and expose actual seek positions, fallback scans
  and required preroll. Container access points do not prove exact codec state.
- Keep state ownership and continuation explicit. Any portable checkpoint must
  validate its version, source identity and byte position before reuse.
- Keep borrowed and owned packet lifetimes clear; optimize copying only after
  correctness and a matched allocation benchmark establish the benefit.

## Capability boundary

The initial milestone parses ID/size integers from slices. Unreleased work adds
a schema-driven incremental `Read` framing core; its exact structural scope and
limits are described in [streaming.md](streaming.md). It does not yet validate
a complete WebM document. A separate [block parser](blocks.md) splits complete
Block/SimpleBlock payloads without container-level track or timestamp interpretation.
ID widths are limited to four bytes and sizes to eight for the intended Matroska/WebM scope. It follows
[RFC 8794](https://www.rfc-editor.org/rfc/rfc8794.html), including the
[RFC 9559 correction](https://www.rfc-editor.org/rfc/rfc9559.html#section-4.2)
that permits ID 0x80. Header-declared maxima, element schema membership, payloads
and hierarchy are not validated by these integer functions.

The intended read contract covers tracks, clusters, SimpleBlock/BlockGroup,
all lacing modes, packet timestamps, CodecPrivate, CodecDelay, SeekPreRoll and
DiscardPadding. General Matroska extensions must be supported explicitly or
rejected/skipped according to a documented rule. Writing containers is a separate
capability and must distinguish live output from durably finalized indexed files.
Sequential audio events now integrate the core and block parser under the
explicit [demuxing subset](demuxing.md). Broader end-to-end qualification
remains scoped; the target contracts are broader
than current verified capability.

The [positional source layer](sources.md) is separate from sequential `Read`:
providers supply a known complete length and immutable revision, reads have
explicit per-operation work budgets, and read extents do not imply validation.
The separate [seek reader](seeking.md) validates structural destinations, reports
actual starts/preroll/fallback and scopes completion to the suffix it reads.
Skipped media and decoder state remain unverified.

## Evidence for decisions

The selected competitors and the mechanisms worth evaluating are documented in
[comparison.md](comparison.md). API compatibility, correctness, streaming and
resource behavior must each be tested independently. No single benchmark or
synthetic packet proves overall superiority. The [qualification protocol](qualification.md)
defines the boundary for future claims. Backlog.md is the only execution queue;
this document records stable requirements, not task status.

The [writer](writing.md) accepts already encoded WebM blocks over Write
with separate Write + Seek finalization. Live output retains source-completeness
uncertainty; indexed output requires a checked receipt. Generic failed flushes
cannot guarantee atomic visibility, so the file example stages publication.
[Output evidence](writer-qualification.md) covers bounded multi-block Cluster
packing and independent long/mixed output checks, retains the initial layout
limitation and reports a laced-padding playback disagreement.
