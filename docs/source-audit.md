# Source audit for the private 0.2.0 milestone

Reviewed locally on 2026-11-02 from `bdbf50b` through the fixes in this milestone.
This is a detailed internal source/contract review with regressions and retained
corpus evidence. It is not an independent security audit or production approval.
Raw findings, rejected hypotheses, commands, logs and profiles stay ignored under
`runs/release-audit`; accepted sanitized conclusions are recorded here.

## Confirmed defects and refactoring

Two behavior defects were reproduced before correction. Three failing test runs
capture reader/writer delay behavior and CodecID string validation.

| Finding | Trigger and previous behavior | Correction and evidence |
| --- | --- | --- |
| Opus delay lower boundary | Pre-skip 312 gives exactly 6,500,000 ns. Both APIs accepted 6,499,999, despite the documented strictly subnanosecond tolerance. Positive one-nanosecond error was already refused. | Shared allocation-free OpusHead relation check uses a symmetric strict bound. Reader and writer regressions fail before the fix and pass after it. Every u16 pre-skip is checked independently against its rational floor/ceiling, including neighboring rejected integers and u64::MAX. |
| Nonprintable CodecID | Control bytes 1, 9, 10, 31 and 127 before a null terminator passed ASCII validation and reached finalized metadata. | Require EBML String's printable range 0x20 through 0x7e before its terminator. Reader regression fails before the fix. Printable endpoints and arbitrary bytes after the first null remain valid according to the termination rule. |

The ASCII rule follows [RFC 8794 section 7.4](https://www.rfc-editor.org/rfc/rfc8794.html#section-7.4);
[section 13](https://www.rfc-editor.org/rfc/rfc8794.html#section-13) requires ignoring
post-terminator bytes. UTF-8 metadata is still validated as UTF-8, not restricted
to ASCII. No codec bytes, raw clocks or trim data are rewritten.

The refactoring removes duplicated reader/writer pre-skip arithmetic and duplicated
finite Cluster header assembly. Existing packet, allocation, failure-state and
wire-output tests cover those paths. This is a focused simplification; no new
framework, dependency, recovery mode or broad parser rewrite is introduced.

## Hypothesis rejected by real corpus evidence

The current [Opus mapping](https://www.matroska.org/technical/codec_specs.html#a_opus)
uses mandatory-presence wording for CodecDelay. An attempted physical-presence
check rejected both native Chromium MediaRecorder fixtures: each has zero pre-skip
and omits the field. The previous documented reader policy interprets the zero
default and refuses absent nonzero delay instead of deriving one from CodecPrivate.
That policy is preserved explicitly, with a regression, rather than silently
changing browser support during refactoring. The attempted restriction is not
reported as a fixed defect or part of the final source. Full mapping conformance
under every interpretation is not claimed. An application needing physical field
presence would require a separately specified strict-conformance capability.

Standalone OpusHead accepts unspecified input rate zero. Integrated metadata
continues to require the documented positive SamplingFrequency/header relation;
it does not invent a replacement sample rate. Decoder rate remains a consumer
choice. Mapping families beyond the supported subset remain explicit errors.

## Module coverage

| Source | Reviewed invariants and checks | Outcome and residual boundary |
| --- | --- | --- |
| lib.rs | VINT marker/width, canonical IDs, size sentinel, checked slice bounds | Existing exhaustive one/two-byte tests and all widths; integer syntax does not prove schema validity. |
| block.rs | Track VINT, flags, all laces, 256-entry stack table, length partitions and Xiph work cap | Fixed storage and checked lengths preserve frame views; no codec parsing or invented later timestamps. |
| stream.rs | Finite/inherited boundaries, unknown-master termination, lookahead, poisoned errors and read budgets | Original every-byte fault/continuation tests retained; trusted-schema lookup cost and provider latency are outside byte budgets. |
| demux/schema.rs | Supported parents/IDs, singleton/repeatable fields, globals and opaque skips | No payload pattern resynchronization; skipped master contents remain unvalidated. |
| demux/mod.rs | Metadata aggregate limits, track selection, late group fields, packet lifetimes, EOF/completeness and exact clock arithmetic | CodecID control-byte defect fixed. Every real producer and generated comparison case retains packet identity. |
| demux/timing.rs | Located scalar/header causes, delay/pre-skip relation and whole-block padding duration | Negative one-nanosecond defect fixed through common helper; zero-default interoperability policy remains explicit. |
| opus.rs | All header widths/versions/maps, informational input rate and exact rational pre-skip | New exhaustive delay oracle complements existing header/channel-map checks; no codec packet validation. |
| source.rs | Snapshot before/after reads, latched replacement, count validation, extents and per-operation accounting | Existing A-B-A and wrong-provider tests retained; same-length writes cannot be detected by a generic Read + Seek adapter. |
| seek/index.rs | Actual Segment/Cluster boundaries, checked offsets, bounded transactional indexes, cue track/time/relative destinations, fallback and cancellation | Index success verifies selected prefixes, not all skipped media, packet payload validity or decoder readiness. |
| seek/mod.rs | Atomic cursor replacement, pending packets across cancellation/range holes, source checks before publication and suffix End | Existing preparation/suffix every-byte faults and forward/backward tests retained; blocking provider calls need application latency bounds. |
| write/ebml.rs | Finite size/VINT widths, fallible bounded serialization, master patching and small headers | Deduplicated Cluster header construction; unchanged output identities. IDs are internal constants, not caller-controlled serialization IDs. |
| write/mod.rs | Preflight versus sink failure, monotonic exact clocks, lacing/group metadata, cue count/bytes, packed/direct paths, flushing and finite patch sequence | Shared delay defect fixed. Generic sink atomicity remains impossible after a final flush failure; explicit receipts and staged publication remain required. |
| Examples and dependency tool | Selected-track manifests, same-reader continuation, seek reporting, staged no-replace publication and locked metadata | No normal build downloads, third-party dependencies, unsafe code or production unwrap/expect. External qualification stays optional/local. |

Bounds are examined at construction, parsing and state-transition boundaries.
No new silent skip, rounding, trimming, default resource increase or relaxation
of completeness was introduced. A rejected block is not retried after partial
I/O. The existing bounded live buffering behavior and migration remain explicit.

## Verification and evidence scope

The final Rust 1.89.0 gates cover formatting, strict all-target Clippy, offline
locked debug/release tests, warning-free rustdoc and the dependency checker.
There are 143 integration tests, two unit tests and five doctests. The exhaustive
delay test checks all 65,536 pre-skip values against independently calculated
adjacent integers.

A fresh bounded five-mode reader/source/seek campaign uses seed 6110202, 2,000
mutations plus 156 seed replays (2,156 executions) from the retained preliminary
corpus. It completes
without a crash, timeout or oracle mismatch. Exact replay counts and source/tool
hashes are retained in its manifest. Coverage feedback is source-region feedback,
not branch coverage; the writer is exercised by the existing generated/fault
suite, not a new coverage-guided writer campaign. A preliminary campaign used a
rejected presence-check hypothesis and is not attributed to final-source behavior.

All 25 historical selected-audio inputs match the old complete manifests for
selected bytes/order/track, exact clocks, laces, CodecPrivate and signed whole-block
padding. The replay normalizes the manifest adapter's whole-block padding field
to the public example's explicit block field; it does not apply padding twice.
All 14 selected-audio writer outputs reproduce their qualified SHA-256 exactly,
including six-hour, browser, YouTube and Vorbis cases. No old timing artifact or
manifest was overwritten. Current performance is measured separately after this
source is committed; a passing replay is not a speed claim.

## Remaining qualification risks

- This internal review is not independent review, broad security certification or
  a guarantee against unexercised parser/state bugs. Allocation-failure behavior
  is fallible but exhaustive allocator-fault injection is absent.
- Coverage-guided writer mutation, longer campaigns, additional architectures and
  operating systems, and diverse natural multi-hour recordings remain unqualified.
- CRC verification and general Matroska transformations/state/metadata are outside
  the explicit subset. Content that requires them fails; opaque skipped elements
  do not establish their internal validity.
- The six-hour before-first-cue fallback exceeds default source-call limits;
  explicit 500,000-call qualification is separate from default admission.
- Ordinary pinned FFmpeg playback repeats whole-block padding on laced frames.
  Independent grouped decode agrees with the preserved metadata, but ordinary
  playback interoperability is not universal.
- Public distribution still requires an owner-selected license and explicit
  publication authorization. A source version and successful tests do not supply
  either or establish production suitability.
