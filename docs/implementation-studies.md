# Pinned implementation studies

This supplements the initial [selection review](comparison.md) and the
[matched measurements](matched-comparison.md). Research completed on 2026-11-01
UTC; raw sources, licenses, upstream fixtures, build logs and original probes
remain local in ignored research directories. No competitor code or fixture is
incorporated into unlace. Source/test inspection is distinguished from executing
an upstream suite. Focused behavior probes are not matched benchmarks.

## Implementations in the matched report

The exact five library pins and build artifacts are recorded in the matched
report. Its complete manifests, native End gates, repeated timings and seek
checks provide executed behavior evidence in addition to the source/test review.

| Project | Source and tests studied | Mechanism and unlace consequence |
| --- | --- | --- |
| libwebm | mkvparser, mkvmuxer and separate webm_parser; parser BlockGroup/partial-status tests and muxer finalization/Cues tests | Keep incremental framing separate from full-load indexing; wait for late group metadata; make writer finalization explicit. Both parser APIs are measured separately. |
| Nestegg | Public header, nestegg.c, regress.c short/fake-EOF/failed-seek callbacks and indexing.c | Small packet API and explicit raw delay/preroll/padding; preserve located errors and distinguish source EOF from stronger completeness. |
| libavformat | matroskadec.c, matroskaenc.c and FATE Matroska lacing/remux/unknown-Cluster/transform cases | Rational native timestamps and sample-side trim need field-by-field admission; unsupported transforms must remain explicit. Decoder readiness is separate from container access. |
| Symphonia | Pinned format-mkv demuxer, EBML iterator and lacing modules; registry archive inspected for package-local tests, with independent corpus execution supplying behavior evidence | Framework track/packet ownership, timestamp units and validated seek element types are useful; codec normalization differs from original CodecPrivate preservation. No complete upstream suite run is claimed. |
| matroska-demuxer | src/lib.rs and tests/lib.rs oversized strings, unknown-size masters, packet/seek fixtures | A dependency-free Read + Seek API is practical; exact metadata and sequential input still require their own contracts. Existing public Frame trim limitations are measured rather than guessed. |

No full upstream suite execution is claimed for these five. The matched report
records the independently written adapters and their failures as well as passes.
Libwebm's muxer is studied as a separate writer implementation, not mistaken for
its incremental parser. Source paths refer to the exact pins in that report.

## libmatroska and libebml

Pins: [libmatroska 68e9391](https://github.com/Matroska-Org/libmatroska/tree/68e939111c77cce82667bf1f87bbf718b92a6515)
and [libebml 1f64c9a](https://github.com/Matroska-Org/libebml/tree/1f64c9a1581875cb59571c6e1bd62544dfdc3fd3),
both development manifests identify 2.0.0. Reviewed EbmlMaster finite/unknown
handling, semantic contexts, KaxBlock/KaxCluster, Cues/SeekHead, libebml's infinite/
header/missing/version tests and libmatroska's mux/read/tree examples.

Build: GCC 15.2.0 C++17 Release, CMake 4.4.4, Ninja 1.13.2, static libraries,
libebml BUILD_TESTING=ON and libmatroska BUILD_EXAMPLES=ON. The build fetches
utf8cpp v3.2.5 at `6f0e7c7865208f2a6b882a7e138584beb1b6b2fd`. All eight libebml CTest
cases pass. Libmatroska's bundled test6 mux example reproducibly exits on signal
11 before reading any unlace input. This is a narrow observed example failure,
not attribution of a general parser defect. Its mkvtree exits zero and traverses
our indexed mono, live mono and signed-padding EBML-laced outputs. Tree acceptance
is not packet equivalence. A 512 MiB/10 CPU-second/15 wall-second bound applies
to those probes; later repetitions disable core dumps.

The useful mechanism is schema-specific unknown-size permission and explicit
master ownership. Unlace's schema and checked writer receipt keep that distinction
without adopting a broad mutable object tree or claiming these examples prove
its supported semantics. A stable-release comparison would be a separate study.

## GStreamer matroskademux

Source: [gst-plugins-good 1.28.2](https://gstreamer.freedesktop.org/src/gst-plugins-good/gst-plugins-good-1.28.2.tar.xz),
archive SHA-256 `1ace2d8ec74f632d82eab5006753a27fe0c2402db4ca94d63271e494b62f50bf`.
Installed runtime/plugin report 1.28.2, Ubuntu package 1.28.2-2ubuntu0.4.
The libgstmatroska.so SHA-256 is
`7084be6294838686ce73d938f9114011297da9341cc1bef6ba3802d77127fb83`.
Reviewed matroska-demux.c clock/clipping, index fallback and bounded backtracking,
and tests/check/elements/matroskademux.c subtitle termination, TOC and segment-loop
cases. The upstream suite was inspected, not rebuilt or run.

An original Python/GObject appsink probe consumes filesrc through matroskademux
without decoding. It matches all 52 mono packet payloads and six EBML-laced
payloads and reaches EOS under a 20-second deadline. Initial mono PTS is zero,
versus unlace's -6,500,000 ns. The source explicitly clamps timestamps below delay
and supplies clipping metadata. GstAudio introspection is unavailable in this
probe, so raw signed block-trim equivalence is untested, not silently zero.

The [documented backtrack bound](https://gstreamer.freedesktop.org/documentation/matroska/matroskademux.html)
is a useful pattern: expose fallback limits and actual behavior. Current online
docs also describe newer properties; they are not attributed to the tested
1.28.2 binary. Unlace exposes its own byte/call bounds rather than copying a
playback pipeline's timestamp normalization.

## webm-iterable

Pin: [116d591](https://github.com/austinleroy/webm-iterable/tree/116d5917dcfeb190335b85177a66751ec2084d6a),
manifest 0.7.1. Reviewed the Read iterator/writer aliases, optional asynchronous
iterator, selective Master::Full grouping, Block/SimpleBlock lace conversion and
the tag round-trip/fixed/Xiph/EBML tests. Its API separates typed EBML elements
from consumer packet semantics; preserving a whole selected master is convenient
but requires an explicit memory policy.

An isolated Rust 1.89 probe builds against that exact source and compares every
extracted payload byte: 52 mono frames and six EBML-laced frames match. This is
element/lace extraction, not an exact timestamp/trim/completion contract.
The attempted upstream library test build cannot finish because networking
example dev dependencies require missing OpenSSL development configuration;
that build failure is not a parser failure. It remains recorded rather than
removing upstream dependencies and calling the modified suite a pass.

Unlace retains its borrowed frame parser and explicit group-level metadata;
container timing and bounded aggregate retention stay in separate layers.

## Mediabunny

Pin: [v1.65.0, cd66123](https://github.com/Vanilagy/mediabunny/tree/cd6612380c7e3f1261d74b9b550e6fc3b65a53bc).
The npm 1.65.0 archive SHA-256 is
`807282601ac6e6c7c72ad8d0a51de7b4a9335a9976582ec10b5b545ba311ffc9`;
registry SHA-512 integrity is verified. A redundant full clone was cancelled
after the pinned package/source/test evidence was obtained. Packaged demuxer source matches the pinned
tag byte for byte. Reviewed Matroska demux/mux, asynchronous source/packet APIs,
lookup-target claims and lace expansion. Pinned upstream demux tests cover zero
integers, zero TimestampScale and concurrent metadata reads; mux tests cover
negative starts and CodecDelay. They were inspected, not run as a full suite.

An original Node 22.19.0 EncodedPacketSink probe matches all 52 mono and six laced
payloads. Mono first time is -0.007 seconds, versus exact -0.0065; the reviewed
source rounds delay to container ticks. Laced packet times advance by 20 ms.
Raw signed whole-block padding is unavailable in the tested public packet path,
so the probe is not admitted as exact-metadata equivalent. BufferSource retains
these small fixtures; it is not a bounded-streaming memory measurement.

Useful mechanisms are awaited writer backpressure and explicit source ownership;
unlace keeps synchronous call limits, whole-block padding and exact i128 clocks.

## ebml-go

Pin: [e3fb7f9](https://github.com/at-wat/ebml-go/tree/e3fb7f90b76d91d81f073b8bc95a567c2962332a).
Reviewed marshal/unmarshal hooks, lacer/block tests, mkvcore block-reader tests,
channel ownership, writer queues and seek-head construction. `go test ./...`
passes with isolated Go 1.27.3, whose Linux archive SHA-256 is
`65efbc80fdc1fa2cd2631cad5b481777a68e8725f34e89f50fc6ac714e379b40`.
No Go tool/dependency enters the unlace Cargo build.

The documented NewSimpleBlockReader convenience path drops optional BlockGroup
data. An original probe matches every byte of the 52 mono and six laced packets.
Its first three laced clocks are 0, 0, 0 ticks, while DefaultDuration establishes
0, 20 and 40 ms. It reports raw block ticks without CodecDelay subtraction; the
mono first clock is zero. This convenience path is therefore not an exact trim/
frame-clock adapter. Lower-level Unmarshal remains a different capability.

The useful mechanism is small io.Reader/io.Writer composition; asynchronous
channels and background error callbacks also demonstrate why ownership and fatal
completion must be explicit. Unlace does not hide asynchronous writer failures
behind a generic EOF or infer later lace times without metadata.

## Reproduction boundary

The supplemental probes are in ignored `runs/objective-audit`, with source pins,
archive hashes, original probe sources and logs. For libebml/libmatroska configure
CMake as above, then run CTest/test6/mkvtree. For Go run the unmodified upstream
suite and the isolated mkvcore reader probe. For webm-iterable build the isolated
Rust probe against the pinned path. For GStreamer connect filesrc, matroskademux
and appsink and require EOS; for Mediabunny iterate EncodedPacketSink to exhaustion.
All small probe payloads are compared against the existing independently checked
writer fixtures. No new superiority claim, full-suite coverage, or interchangeable
validation guarantee follows from this study.
