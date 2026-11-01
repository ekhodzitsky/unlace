# Changelog

Source milestones use SemVer. Entries do not imply a published crate or binary.

## Unreleased

- Add zero-allocation, schema-driven EBML framing over `Read`, with retained
  partial headers, `WouldBlock` continuation and explicit final source EOF.
- Enforce finite parent boundaries and schema-driven unknown-size termination;
  refuse unsupported IDs and invalid forms without scanning payloads.
- Bound depth, declared metadata/payload/skip sizes and per-poll read work.
- Add allocation-free Block/SimpleBlock frame views for no, fixed, Xiph and
  EBML lacing, preserving track number, signed relative timestamp and flags.
  Enforce block/frame/count/lace-work limits with malformed-input regressions.
- Add sequential framing example and deterministic streaming regressions.
  This core does not yet validate WebM documents or extract audio packets.

## 0.1.0 - 2026-11-01

- Establish the private, unpublished, zero-third-party-dependency Rust project.
- Add allocation-free EBML ID and size parsing with explicit incomplete input,
  size sentinels and canonical IDs, including RFC 9559's legal 0x80 ID.
- Add boundary/exhaustive tests, an executable example and local Rust 1.89 checks.
- Record container design, competitor evidence and matched qualification rules.
  Full WebM/Matroska demuxing is not implemented in this milestone.
