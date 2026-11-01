# Original objective audit

Audited against source `2143ba0`, plus the continuation example and evidence
updates in this change. The original implementation and qualification work is
complete for the explicit supported container subset below. This is an
unpublished private source milestone, not production qualification, full
Matroska coverage or a claim of overall superiority. Future support and broader
platform/media campaigns are separate from the verified scope.

## Requirements and evidence

| Original requirement | Delivered contract and verification |
| --- | --- |
| Container library independent of codecs, downloading and recognition | Public framing, block, demux, source, seek, Opus metadata and writer modules. Encoded data is exposed unchanged; external codecs/tools appear only in optional local qualification. [Design](design.md). |
| Pure Rust, no unsafe or third-party Cargo dependencies in any category | Crate forbids unsafe; strict production unwrap/expect lints. Offline locked metadata/dependency checker returns one package, zero dependencies. Rust 1.89.0 gates pass. |
| Study all named implementations through documentation, source, tests and behavior | [Pinned studies](implementation-studies.md) cover all ten named projects, including libwebm's three distinct components and libmatroska/libebml. Source/test inspection and actually executed suites/probes are distinguished. [Matched report](matched-comparison.md) builds the five required competitors. |
| Pin counterparts and respect provenance/licenses | Exact revisions, source/archive/executable hashes, compiler settings and corpus rights in the studies and qualification reports. Original adapters and fixtures; third-party source/licenses retained locally. No borrowed code or media enters ordinary builds/tests. |
| Sequential input without required Seek | `Demuxer<R: Read>` and structural Reader; tiny/short/interrupted reads and every-byte WouldBlock continuation. [Streaming](streaming.md), [demuxing](demuxing.md), executable `audio_packets` and `continue_audio`. |
| Separate seekable/ranged contract | `ReadAt`, immutable Snapshot, RangeReader budgets/NeedRange and `SeekSource<R: Read + Seek>`, separate SeekReader. [Source contract](sources.md). No library networking. |
| EBML header, declared widths, Segment/Info/Tracks/Cluster | Validated header/document requirements, schema hierarchy, defaults/duplicates, supported versions and required metadata. Original truncation/width/boundary regressions. [Demux subset](demuxing.md). |
| SimpleBlock, BlockGroup, every lacing mode | Borrowed complete-block frame views; no/fixed/Xiph/EBML lacing, all 256 slots and late group metadata. Exhaustive flags/counts and generated round trips. [Blocks](blocks.md). |
| Audio selection and safe other-track skipping | First enabled or explicit TrackNumber; unchanged bytes/order/identity, bounded skipped payload/additions. Mixed VP8/VP9 and disabled-first/multiple-audio fixtures plus actual mixed MediaRecorder. |
| Permitted unknown-size masters | Schema-controlled Segment/Cluster termination by actual boundaries; no payload byte-pattern resynchronization. Browser chunk continuation and 10,000 successive unknown Clusters. [Streaming](streaming.md). |
| CodecPrivate, delay, preroll and signed padding | Raw metadata retained, exact i128 ns arithmetic, typed OpusHead validation, delay applied once, whole-block padding across laces. [Opus contract and independent decode proof](opus-timing.md). |
| SeekHead/Cues and exact offsets/clocks | Bounded structural index parsing and actual destination/block verification; checked offset/size arithmetic, forged-offset and i128 endpoint regressions. [Seeking](seeking.md). |
| Explicit supported subset and unsupported transformations | Documented CRC, encryption/compression, additions/state and general metadata limits; explicit unsupported/resource errors, without silent relaxation. [Demuxing](demuxing.md), [writing](writing.md). |
| No whole-file/result retention; bounded hostile work | Per-leaf/block/track/index/nesting and per-poll/source-operation limits, fallible allocations, explicit refusals. Sequential/live state does not grow with duration; indexed state has explicit caps. Six-hour streaming equality processes one packet at a time. |
| Exact packet identity/order/boundaries | Complete native manifests require every packet and End, with no missing/extra data. [Producer corpus](packet-qualification.md), [matched comparison](matched-comparison.md), [writer output](writer-qualification.md). |
| Independent beginning/end sample verification | Original mono/stereo decoder adapters agree on full PCM and first/last 64 frames after initial/final trim. Laced signed trim is checked by an independent whole-block decoder path; normal FFmpeg playback disagreement is retained. |
| Distinct continuation, EOF, corrupt, unsupported, resource and source error | Typed outcomes and terminal/retry scope documented and fault-tested. Earlier packets never turn a later failure into End. [Robustness](robustness.md). |
| Honest completeness uncertainty | Finite Segment/source completion is explicit; unknown Segment cannot prove producer intent. A seek proves only its visited suffix. A writer receipt does not establish original source intent or generic sink durability. |
| Actual seek position, preroll and fallback | Seek outcome reports all three. Repeated/backward suffix and one-second-window comparisons, cancelled/missing-range atomic cursor tests and source identity checks. Container access does not prove decoder state. |
| Writing after read qualification | Live and indexed encoded writers follow the reader/trim/seek qualification work. Separate flush/finish receipts, exact metadata, bounded packed Clusters and oversized direct path. [Writing](writing.md). |
| Interrupted, disk-full and failed finalization | Every-byte write faults, all finalization seeks, interrupted/zero/invalid writes, budgets and flush failures poison output without success. Staged no-replace publication requires checked finish/sync; existing destination and failed staging tests pass. |
| Independent output acceptance and packet equality | All seven remux sources in both modes match classic/incremental libwebm and Nestegg, including all 1,080,001 six-hour packets. Additional mixed VP8/VP9, dual-audio, all laces and scoped RGB/PCM comparisons. Final rebuild reproduces pinned output hashes. |
| Short/multi-hour, browser, YouTube and multiple tracks | Own short and six-hour corpus, native Chromium MediaRecorder chunks, completed licensed Big Buck Bunny YouTube source, mixed/dual tracks. [Packet](packet-qualification.md), [long](robustness.md), [writer](writer-qualification.md) reports. |
| Multiple languages in the qualification protocol | Own English/Russian/German synthesized speech matches exact native packet/trim fields and full source/live/indexed PCM. [Speech report](speech-qualification.md). No recognition or language-quality claim. |
| Every-byte truncations and source faults | Ordinary finite-fixture truncation/header/payload/preparation/suffix byte faults, short/interrupted reads and missing ranges; 1,547 bounded counterpart prefixes distinguish native EOF from completeness. |
| Forged sizes/times/indexes; continuation mismatch | Invalid hierarchy, overflow, forged cue targets, selected-block checks, resource caps and same-length/A-B-A snapshot changes. No portable checkpoint is accepted or exported; owned continuation cannot be imported onto another source. |
| Fuzz critical parsers and state transitions | Five bounded optional coverage-guided reader/source/seek targets: 39,000 mutations, 39,603 executions with replay, original seeds and reducer. Existing minimized regressions stay in normal tests. Writer additionally has 96 generated configurations in both modes. No safety proof or writer coverage-guided campaign claimed. |
| Equivalence before performance | Exact selected payload/track/time/trim admission gates before timing; missing metadata, unsupported precision, failed End and untested composition remain different outcomes. Both required Rust competitors are executed but not timed as equivalent after failing the gate. |
| Startup, throughput, RSS, allocations, reads and seeks | 392 sequential plus 280 fresh-handle seek timing/RSS trials, 144 allocation trials, separate prepare/seek/window reads and first-packet timing. [Matched report](matched-comparison.md). |
| Reproducible pins, commands, cache conditions and unfavorable cases | Hardware/OS/compiler, immutable corpus and executable hashes, warm/advisory-evicted runs, seven repetitions and ranges. Advisory eviction is not proof of physical cold cache. Slow unlace iteration, sparse-cache overfetch and counterpart limitations are retained. |
| Complete consumer examples and ownership/error guidance | `stream_ebml`, `audio_packets`, `continue_audio`, `seek_audio`, `remux_audio` and executable module examples. Borrowed packet lifetime, owned continuation, explicit finish, failure/poison and immutable-source duties documented. |
| English docs, SemVer, changelog and truthful current scope | README, contracts and scoped reports link current behavior separately from plans. Cargo remains unpublished 0.1.0; incompatible reader/writer/config/buffering migration accumulates under the next pre-1.0 minor source milestone. [Version policy](versioning.md). |
| Mandatory local verification | Rust 1.89 fmt, all-target Clippy with denied warnings, debug/release tests, warning-free rustdoc and zero-dependency checker. Final suite has 138 integration tests, one unit test and five doctests; examples build under all-target Clippy. |
| Private main, Backlog only, no publication or Actions | Work on main with Conventional Commits and authorized source pushes. Backlog local/ignored, remoteOperations and autoCommit false; repository private, package publish=false, Actions disabled, no tracked workflow/release artifact. Raw media/adapters/logs/credentials remain local. |

## Material boundaries retained

The supported audio/container subset is deliberate. Arbitrary Matroska
transformations/metadata, general codec initialization validation, decoding,
network access, automatic corruption repair, portable state import and guaranteed
decoder restoration remain outside it. Unsupported inputs fail explicitly.
Additional platforms, natural multi-hour content and longer fuzz campaigns would
broaden evidence; the present results do not claim those were tested.

Generic Write + Seek cannot atomically retract a finite header after a failed
final flush; the test reproduces parseable bytes without a receipt. Applications
requiring publication atomicity use staged output. Directory durability is an
application policy. Whole-block padding is preserved despite the documented
ordinary FFmpeg laced playback disagreement; the independent grouped decode
resolves the format semantics without changing metadata to hide that result.

The before-first-cue scan on the six-hour source exceeds the default call budget.
It returns a resource error; an explicitly raised 500,000-call allowance completes
the recorded probe. This is a documented tradeoff, not a silent relaxation or an
unconditional duration guarantee. No measured dimension is promoted to an overall
library ranking. The objective is not interpreted as requiring fabricated
superiority where the measured implementation is slower.

Raw audit evidence remains in ignored `runs/objective-audit`, with links from the
local Backlog research record to these accepted sanitized reports. Only source,
examples and sanitized documentation are committed; no release is cut.
