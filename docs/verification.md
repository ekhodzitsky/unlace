# Verification boundary

The first source milestone uses Rust 1.89.0, safe Rust and no third-party Cargo
dependencies. The pinned toolchain is the minimum version used for local checks.
The integer example parses an EBML ID and an unknown size; the unreleased
streaming example frames a synthetic header. Neither demuxes a WebM file.

Integer tests cover every supported width, all prefixes of representative
headers, reserved/noncanonical IDs, sentinel sizes, nonminimal legal sizes,
the RFC 9559 0x80 update and exhaustive one/two-byte size inputs. Production
parsing uses bounded loops, does not allocate and returns typed errors.
Sequential synthetic demux checks are described below. Full hostile-media corpus
and representative performance qualification remain unfinished.

The dependency check reads locked offline Cargo metadata and rejects dependency
declarations in every category. Regular tests and rustdoc need no third-party
executables, downloaded fixtures or network. Local checks do not establish
Windows/macOS/ARM parity, fuzz coverage or whole-container conformance.

README standard v1 disposition: purpose, current example, alternatives and limits
are documented. Benchmark/coverage requirements remain incomplete; they must not
be replaced with invented numbers. Download/docs.rs indicators are inapplicable
to an unpublished private crate. Hosted CI is disabled. A distribution license
is pending owner selection. No production or universal-superiority claim is made.

Initial local gates passed on Linux with Rust 1.89.0: five integration tests and
one doctest in both debug and release, strict all-target Clippy, formatting,
warning-free rustdoc, the executable example and the zero-dependency check.

Unreleased streaming checks use independently constructed synthetic fixtures.
Seventeen tests cover exact bytes/order/offsets, all nonempty finite-fixture and
twelve-byte-header truncations, short reads, interrupted/WouldBlock continuation
and source errors at every byte position, malformed parents and inherited finite
boundaries, schema validation, unknown-size termination, opaque skipping, width
and resource limits, all 64 stack slots, and 100,000 successive root elements.
A deterministic 12,800-input random smoke test exercises parser entry states; it
is not sustained coverage-guided fuzzing or evidence of whole-container safety.
Full media-corpus, decoded trimming and performance qualification remain
uncompleted; the selected-packet evidence below has a narrower explicit scope. See [streaming.md](streaming.md) for the exact core contract.

Ten additional block tests cover all four lacing modes, independent RFC wire
examples, original borrowed bytes and order, all track VINT widths, signed
timestamp endpoints, reserved flags, all 256 frame slots, empty frames, malformed
lengths/deltas, lace-work and size limits. Variable laces round-trip independently
encoded frame vectors for counts 2 through 256. Exhaustive 65,536 flag/count pairs
and 12,800 deterministic random inputs exercise additional bounded cases. These
are structural tests; they do not qualify codec payloads or audio completeness.
The block example in rustdoc is executable. Container-level demux is exercised
below; the pinned MediaRecorder/YouTube selected-packet comparison is described in
[packet-qualification.md](packet-qualification.md).

Twenty-one sequential demux tests cover original packet bytes/order/track identity,
all lacing modes, signed timestamps and overflow, default-valued empty elements,
late BlockGroup trim/reference/duration fields, every finite-fixture truncation,
every-byte WouldBlock/Interrupted/source errors, declared widths, duplicates,
unsupported transformations/CRC/state, resource/work limits and 10,000 successive
unknown-size Clusters. These are original synthetic fixtures with no decoder or
external tools in ordinary tests. The audio_packets example reads a source
sequentially and requires final EOF before emitting a successful End event.

Initial local external checks with FFmpeg 7.0.2-static (libavformat 61.1.100)
match ordered packet SHA-256 and CodecPrivate on three generated files: Opus
(52 packets), Vorbis live output (21), and mixed VP9/Opus (21). Live output reports
unknown-size completion uncertainty. The same ordered packet SHA-256 values also
match Mozilla Nestegg commit `767aab25013acefbdcc6d68a2a7a2c9081303e3f` on all
three files. This is limited synthetic-corpus evidence from two implementations,
with the additional producer cases documented below. FFprobe's
millisecond time base rounds the first Opus timestamp differently from exact
nanosecond CodecDelay subtraction; no exact timing equivalence is claimed.
Raw media, manifests and command evidence remain local under ignored runs/.

The preceding packet-qualification gates passed locally on Linux with Rust 1.89.0: 53
integration tests and two doctests in debug/release, strict all-target Clippy,
formatting, warning-free rustdoc and zero third-party dependencies. This evidence
does not close the outstanding corpus, timing, trim, fuzz or performance gates.

The real producer corpus now includes native Chromium 156.0.8078.4 MediaRecorder
Opus audio and VP9/Opus output (28 selected audio packets each), plus completed
YouTube format 251 audio for Big Buck Bunny (29,825 packets). Exact bytes/order
match pinned Nestegg; packet SHA-256 and CodecPrivate match FFmpeg. Both actual
browser chunk sequences continue through WouldBlock without packet changes.
The mixed browser case produced failing regressions before adding explicit
MaxBlockAdditionID handling and bounded unselected-track additions skipping.
Selected additions still fail before packet emission. See the sanitized
[packet report](packet-qualification.md) for source rights, input hashes, exact
versions, method and remaining limits. Raw source/profiles/results stay ignored.

Opus initialization/timing adds six header tests and twelve integrated regressions.
They exercise every header truncation/magic byte, all version bytes, all 65,536
stream/coupling count pairs and all u16 pre-skip values, explicit mapping/rate/gain
fields, unsupported mappings, exact/quantized delays, located conflicts/duplicates,
missing initialization/Audio, signed padding across laces, known-duration excess,
and WouldBlock at every position of an Opus-initialized fixture. Generic container
fixtures use A_TEST for deliberately invented packet/private bytes, preserving
their original boundary/timestamp checks without claiming they are valid Opus.

External-only libopus 1.6.1 qualification compares every decoded PCM byte from two
paths for original 1.03-second mono/stereo sources. Initial pre-skip 312 and final
padding 168 samples yield exactly 49,440 frames/channel and identical complete,
first-64 and last-64 sample hashes. The reference derives trim independently from
libavformat WebM extradata/packet side data. An unfavorable Ogg stream-copy result
(49,608 frames) is retained rather than normalized into the expected result. See
[opus-timing.md](opus-timing.md) for input/output hashes, method, API changes and
scope. No codec tool/library is an ordinary build/test dependency.

Final Opus timing gates passed on Linux with Rust 1.89.0: 71 integration tests
and three doctests in debug and release, strict all-target Clippy, formatting,
warning-free rustdoc and the zero-dependency check. The final audio manifest
example was rebuilt; decoded PCM qualification and all three real producer
packet comparisons passed again against that final implementation.

Positional input adds sixteen source-contract tests and an executable module
example. They cover backward/short reads, range continuation at every byte
prefix, bounded retries/work, capped missing-range requests, exact read counters,
explicit extent-log truncation, observed length/revision replacement including
A-B-A, changes during successful/error reads, post-read snapshot failure,
premature EOF, invalid provider counts, empty sources and maximum offsets.
The Read + Seek adapter checks returned seek positions and observed length
changes; it cannot detect same-length external writes. These tests exercise
input infrastructure only, not Cues/SeekHead validation or container seeking.
All required local Rust 1.89.0 gates passed with 87 integration tests and four
doctests in debug/release, strict Clippy, formatting, rustdoc and zero dependencies.

Eighteen container-seek regressions cover attested index destinations, exact
forward/backward packet suffixes, missing/unselected cues, BlockGroup/no-relative
hints, all track VINT widths, every finite-fixture truncation, every-byte missing
ranges, short/interrupted reads, preparation/packet cancellation, immutable-source
checks, CRC classification and index/scan/work refusals. They preserve regressions
for four defects reproduced before correction. A separate local streaming
adapter matched every suffix byte, all packet fields and requested one-second
intervals for 32 queries across
eight pinned sources, including native MediaRecorder and completed YouTube audio.
Opening, seek and packet I/O are counted separately without claiming speed,
network savings, whole-source validation or decoder readiness. See
[seeking.md](seeking.md) for source identities, method, scope and aggregate results.

Final seek gates passed locally with Rust 1.89.0: 105 integration tests and four
executable doctests in debug/release, all-target strict Clippy, formatting,
warning-free rustdoc and zero third-party dependencies. The final file seek
example was built and exercised on the completed YouTube source. All 32 local
suffix/window comparisons passed again against the final library, with unchanged
provider-read counters.

The [robustness qualification](robustness.md) adds optional coverage-guided fuzz
tools, 39,000 mutations with bounded process/oracle scope, library-only source
region coverage, every-byte positional I/O errors and a minimized work-refusal
shape. An original six-hour Opus container matches 1,080,001 selected packets with
two independent demuxers, exact generated timestamp sequence and final trim/EOF.
The default early seek read-budget refusal remains explicit; a larger documented
allowance permits matched forward/backward suffix and one-second-window checks.
These are correctness/coverage observations, not performance or safety claims.

Final robustness gates passed on Rust 1.89.0: 108 integration tests and four
doctests in debug/release, strict all-target Clippy, fmt, warning-free rustdoc
and the zero-dependency check. Optional fuzz tools also passed Rust formatting,
Python byte compilation/CLI checks, warning-free standalone test compilation,
retained-corpus replay, dense-lace cases and reducer self-check. No production
API, Cargo version or runtime dependency changed in this qualification slice.

The [matched selected-audio report](matched-comparison.md) records 150 counterpart
field comparisons across 25 sources/cases, 392 sequential and 280 fresh-handle
seek timing/RSS trials, 84 sequential and 60 seek allocation trials, and 1,547
bounded finite-prefix processes. All five named libraries were built; missing
metadata, unsuccessful completion, unsupported index layouts and unqualified
timing adapters are distinguished. Every admitted measurement matches its
manifest-derived consumer digest/count/window and observed preroll requirement.
Native EOF conventions remain separate from unlace completeness. Raw sources,
adapters, logs and media stay ignored; only sanitized methods/aggregates are
tracked. This slice changes documentation, with no production API, Cargo version
or dependency change.

Final comparison gates passed locally on Rust 1.89.0: 108 integration tests and
four doctests in both debug/release, strict all-target Clippy, formatting,
warning-free rustdoc and zero third-party dependencies. The retained final
adapter rebuild reproduces all timed executable SHA-256 values; an audit verifies
all admitted trial counts/digests/preroll, corpus hashes and measured artifact
pins. No remote workflow or release artifact was created.

Initial encoded output adds 22 writer regressions and one executable module
example. The [writer contract](writing.md) and [partial qualification](writer-qualification.md)
distinguish checked live/indexed receipts, preflight resource refusals, terminal
partial I/O, exact trim/clock handling and staged file publication. Short/native
producer/Vorbis/six-hour audio output passes the documented counterparts within
the stated limits; complete mono/stereo independent PCM is unchanged. Classic
full-load long-output resource failures remain recorded and writer qualification
stays open for bounded Cluster packing and broader multi-track/fault checks.

All initial writer gates pass locally with Rust 1.89.0: 130 integration tests and
five doctests in debug/release, strict all-target Clippy, formatting, warning-free
rustdoc and zero third-party dependencies. Rebuilding the final source reproduces
all 14 qualified output hashes byte for byte. Publication failure checks preserve
staging data and existing destination contents. Cargo stays unpublished 0.1.0;
this additive API joins the pending source milestone without a release/tag.
