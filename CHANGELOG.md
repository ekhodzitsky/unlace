# Changelog

Source milestones use SemVer. Entries do not imply a published crate or binary.

## Unreleased

- Record 1,484 paired time/RSS and 324 allocation trials for 0.2.1 against
  preserved 0.2.0 and pinned readers, including natural speech and large mixed
  inputs. Retain source/clock gates, cache effects, slower competitor cases,
  small-input regressions, broad ranges and the 8 KiB memory tradeoff.

## 0.2.1 - 2026-11-02 (unpublished source milestone)

- Reuse the reader's checked frame ranges and initialized I/O scratch instead of
  rebuilding large buffers for each block/poll. Preserve parsing, timing, trim,
  limits and completion contracts. The 8 KiB scratch is now retained per reader;
  no heap allocation or public API requirement is added.

- Qualify 5.39 hours of licensed natural speech and a physical 2.92 GB mixed
  container with complete bounded native packet, decoded-span, continuation and
  seek checks. Preserve fixture mux changes and the default mixed-seek budget
  refusal explicitly; no application SLA or broad production claim follows.

- Strengthen writer fuzzing with a guaranteed-valid acceptance lane and portable
  oracle controls after a second-agent review reproduced missed false-refusal
  and terminal-state mutants. Enforce successful/failed/preflight state classes
  and record 18,000 follow-up mutations separately from production defects.

- Add an optional bounded writer state fuzz target with caller-record/output,
  preflight and terminal-I/O oracles, deterministic sink faults and separate
  versioned replay. Record native packet/seek checks and explicit campaign gaps;
  normal builds still need no extra tools or dependencies.

- Record separate public-release and production readiness gates. Local packaging
  succeeds; license/publication decisions and broader production qualification
  remain open. No public release, license grant or visibility change is implied.

- Refresh performance comparisons for audited 0.2.0: 952 time/RSS and 210
  separate allocation trials, strict metadata and payload-only Rust admission,
  a common seek-aware input cache, and synthetic writer layout cost. Preserve
  adapter-induced overreads, slower long-file iteration and all trial ranges.

## 0.2.0 - 2026-11-02 (unpublished source milestone)

- Reject Opus CodecDelay values whose difference from exact pre-skip is a whole
  nanosecond or greater in either direction. Fix the previously accepted negative
  one-nanosecond boundary in both reader and writer through one shared check.
- Require printable ASCII before the first null terminator in EBML String fields,
  including CodecID. Preserve valid termination semantics and UTF-8 field rules.
- Deduplicate writer Cluster header serialization. Record a module-by-module
  audit, exhaustive delay-boundary regression and repeated corpus/fuzz evidence.
- Cut the private 0.2.0 source milestone for the accumulated reader/writer API.
  This is not a public release, license grant, tag or package publication.

- Add a tiny-chunk continuation example, pinned supplemental implementation
  studies, own multilingual speech packet/PCM checks and a requirement audit.
  Preserve explicit unsupported/failure observations and existing benchmark scope.

- Add initial encoded WebM writing with separate live and indexed output, all
  lacing modes, exact clock/delay/trim preservation, bounded fallible state and
  explicit finish receipts. Poison partial I/O and retain failure stage/cause;
  generic flush success is not atomic publication or storage durability.
- Add a staged selected-audio remux example and writer regressions covering
  every-byte output failure, finalization faults and verified cue destinations.
  Independent packet, mixed VP8/VP9, multiple-audio and scoped decoded checks
  pass; record an ordinary FFmpeg laced-padding playback disagreement explicitly.
- Pack finite Clusters within explicit byte/time/tick bounds, with a direct path
  for oversized blocks, explicit flush and separate block/Cluster counts. The
  six-hour output now completes classic libwebm under the original memory cap.
  Breaking relative to the initial writer: configuration/receipt fields expand
  and default block writes buffer one Cluster until rollover, flush or finish.
  See the writing migration notes; zero packing bounds restore direct emission.
  These incompatible additions are included in this 0.2.0 source milestone.

- Record pinned selected-audio comparisons against five competitor libraries,
  including separate classic/incremental libwebm adapters. Gate exact fields
  before timing; report repeated startup/throughput, process RSS/allocations,
  read volume, seek windows and unfavorable cases without an overall ranking.

- Add optional dependency-free coverage-guided fuzz tools with original synthetic
  seeds, bounded input processes, retained findings and a bounded reducer. Record
  exact scope/coverage and 39,000 mutations without treating success as safety.
- Extend every-byte positional I/O and work-refusal regressions. Qualify all
  1,080,001 packets and exact timeline/final trim of an original six-hour Opus
  container; retain the default before-first-cue seek budget refusal explicitly.

- Add separate selected-audio seeking over immutable positional sources: attest
  real Segment/Cluster boundaries, parse bounded Cues/SeekHead and verify used
  cue destinations against actual selected-track blocks before restarting.
  Report actual time/position, declared/effective preroll, explicit scan fallback
  and suffix-only completion; skipped media and decoder state stay unverified.
- Preserve packet state across cancelled/unavailable seek requests. Add a file
  seek example, hostile/ranged-input regressions and 32 exact suffix comparisons
  across eight pinned sources, including native MediaRecorder and YouTube.
  No matched competitor seek or performance claim is made.

- Add bounded positional input over provider-versioned immutable snapshots and
  a separate Read + Seek adapter. Report capped missing-range requests, actual
  read work and explicit extent-log truncation; latch observed source changes
  and inconsistent EOF/read counts. This is input infrastructure, not yet a
  container seek API or index validator.

- Validate A_OPUS initialization, channel/rate/delay relations and exact pre-skip
  samples; expose compatible OpusHead fields without decoding or changing bytes.
  Report integer-nanosecond quantization explicitly instead of silently rounding.
- Add located typed Timing errors for malformed/conflicting initialization and
  timing metadata. Preserve whole-block signed padding across all laced frames;
  refuse padding beyond a known block duration before emitting packets.
- Qualify exact mono/stereo decoded start/end spans with independent libavformat
  trim metadata and the same external libopus decoder, with no Cargo dependency.
- API migration from the initial integer-only milestone: Track literals require
  `opus_timing`, Packet literals need `block_discard_padding_ns`, and DemuxError
  matches need Timing. Generic timing Invalid errors become located Timing
  causes. These incompatible changes are included in 0.2.0; the original 0.1.0
  stays unpublished.

- Preserve MaxBlockAdditionID track declarations and safely skip bounded opaque
  BlockAdditions on unselected tracks. Selected-track additions remain explicit
  unsupported errors, including when additions precede the group's Block.
  Track struct literals must initialize the new `max_block_addition_id` field
  (default metadata value 0); this incompatible addition is included in 0.2.0.
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
