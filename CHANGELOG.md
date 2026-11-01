# Changelog

Source milestones use SemVer. Entries do not imply a published crate or binary.

## Unreleased

- Preserve MaxBlockAdditionID track declarations and safely skip bounded opaque
  BlockAdditions on unselected tracks. Selected-track additions remain explicit
  unsupported errors, including when additions precede the group's Block.
  Track struct literals must initialize the new `max_block_addition_id` field
  (default metadata value 0); account for this pending API change at the next
  source milestone under the pre-1.0 minor-version policy.
- Qualify selected audio packet bytes/order and CodecPrivate on pinned native
  Chromium MediaRecorder and completed YouTube audio against FFmpeg and Nestegg.
  Record sanitized corpus identities and scope; raw media/results remain local.

- Add bounded sequential selected-audio packet events over `Read`, with stable
  track identity, codec metadata, checked signed nanosecond timing and explicit
  uncertainty for later laced frames lacking DefaultDuration.
- Retain BlockGroup data until late reference/duration/padding metadata is known;
  preserve encoded packet bytes and apply CodecDelay to timestamps exactly once.
- Enforce aggregate metadata/track/work limits and supported document constraints;
  reject unsupported content encodings, codec state/additions and CRC explicitly.
- Add a sequential audio manifest example and container-level regressions for
  every-byte truncation, interrupted input, selection, lacing and terminal errors.
  Decoded trim and broader corpus/performance qualification remain unfinished.
- Add zero-allocation, schema-driven EBML framing over `Read`, with retained
  partial headers, `WouldBlock` continuation and explicit final source EOF.
- Enforce finite parent boundaries and schema-driven unknown-size termination;
  refuse unsupported IDs and invalid forms without scanning payloads.
- Bound depth, declared metadata/payload/skip sizes and per-poll read work.
- Add allocation-free Block/SimpleBlock frame views for no, fixed, Xiph and
  EBML lacing, preserving track number, signed relative timestamp and flags.
  Enforce block/frame/count/lace-work limits with malformed-input regressions.
- Add sequential framing example and deterministic streaming regressions.
  The framing/block cores remain available separately from the demuxing subset.

## 0.1.0 - 2026-11-01

- Establish the private, unpublished, zero-third-party-dependency Rust project.
- Add allocation-free EBML ID and size parsing with explicit incomplete input,
  size sentinels and canonical IDs, including RFC 9559's legal 0x80 ID.
- Add boundary/exhaustive tests, an executable example and local Rust 1.89 checks.
- Record container design, competitor evidence and matched qualification rules.
  Full WebM/Matroska demuxing is not implemented in this milestone.
