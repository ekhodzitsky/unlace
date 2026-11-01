# Changelog

Source milestones use SemVer. Entries do not imply a published crate or binary.

## Unreleased

## 0.1.0 - 2026-11-01

- Establish the private, unpublished, zero-third-party-dependency Rust project.
- Add allocation-free EBML ID and size parsing with explicit incomplete input,
  size sentinels and canonical IDs, including RFC 9559's legal 0x80 ID.
- Add boundary/exhaustive tests, an executable example and local Rust 1.89 checks.
- Record container design, competitor evidence and matched qualification rules.
  Full WebM/Matroska demuxing is not implemented in this milestone.
