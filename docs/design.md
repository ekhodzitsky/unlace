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

The current 0.2.4 source provides allocation-free EBML integer parsing,
schema-driven framing over Read, standalone Block/SimpleBlock splitting,
sequential selected-audio demuxing, positional sources, verified seeking and
source-bound prepared sessions. The [contract index](README.md#current-contracts)
defines each layer's exact scope. Writing accepts already encoded blocks over
Write, with checked indexed finalization over Write + Seek. Storage durability
and atomic publication remain consumer responsibilities.

ID widths are limited to four bytes and size widths to eight for the intended
Matroska/WebM scope. Integer syntax follows
[RFC 8794](https://www.rfc-editor.org/rfc/rfc8794.html), including the
[RFC 9559 correction](https://www.rfc-editor.org/rfc/rfc9559.html#section-4.2)
that permits ID 0x80. Integer functions alone do not validate header-declared
maxima, schema membership, hierarchy or payloads.

Audio contracts cover tracks, Clusters, SimpleBlock/BlockGroup, all lacing modes,
packet timestamps, CodecPrivate, CodecDelay, SeekPreRoll and DiscardPadding.
General Matroska extensions require explicit support or a documented refusal/
skipping rule. Skipped media and decoder state remain unverified.

Positional providers supply a known complete length and immutable revision.
Per-operation read budgets and required extents do not imply whole-source
validation. Prepared sessions retain the original provider and complete fallback
verification, with no independent-provider import or persistent disk index.

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
