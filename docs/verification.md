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

All current repository gates pass locally on Linux with Rust 1.89.0: 53
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
