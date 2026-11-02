# unlace

Unlace is a private Rust project for bounded WebM and Matroska container reading
and writing.
The **0.2.0 private source milestone** provides allocation-free EBML integers,
bounded sequential audio packets over `Read`, selected-track access and all lacing
modes, verified seeking, and live/indexed WebM writing from encoded blocks.
These supported subsets have scoped independent qualification; unlace is not a
general or production-qualified Matroska reader. The earlier 0.1.0 milestone
contained only integer parsing. No public crate has been released.

[Design](docs/design.md) · [Competitors](docs/comparison.md) · [Verification](docs/verification.md)

The target is a small container library with bounded memory, exact packet and
timing information, sequential input without mandatory seeking, and independently
verified seek behavior. Opus decoding belongs to a separate codec library.

## Try the current source

Rust 1.89.0 and Cargo are required. There are no Cargo dependencies and no
build-time downloads. From this private checkout:

```sh
cargo run --offline --locked --example ebml_header
```

Expected output:

```text
id=0x1a45dfa3 size=unknown
```

The public API returns an explicit incomplete-input result without allocating:

```rust
use unlace::{Parsed, parse_size};

assert_eq!(parse_size(&[0x40]), Ok(Parsed::NeedMore { total: 2 }));
```

An unknown-size result describes integer syntax only. It does not validate
whether the containing element is allowed to have unknown size. A finite size
never authorizes an allocation by itself.

For sequential framing, run `cargo run --offline --locked --example stream_ebml`.
The [streaming contract](docs/streaming.md) explains caller-provided schemas,
zero-allocation payload chunks, `WouldBlock` continuation, final source EOF and
resource limits. The [block contract](docs/blocks.md) covers standalone frame
splitting for all lacing modes. For integrated sequential audio, run:

```sh
cargo run --offline --locked --example audio_packets -- INPUT.webm
cargo run --offline --locked --example audio_packets -- INPUT.webm 2
cargo run --offline --locked --example continue_audio -- INPUT.webm
```

The [demuxing contract](docs/demuxing.md) defines supported elements, borrowed
packet lifetimes, timestamp uncertainty for laced frames, final EOF evidence
and explicit unsupported features. The example emits a correctness manifest;
it does not decode audio or benchmark throughput.

The separate [positional source contract](docs/sources.md) provides bounded
reads for immutable files and externally cached ranges. It reports unavailable
byte extents, actual read work and observed source replacement. This input layer
supports the separate [seek reader](docs/seeking.md), which validates index
destinations and reports actual starts, preroll, fallback and suffix completion.

For selected-audio writing, with no codec encoding:

```sh
cargo run --offline --locked --example remux_audio -- INPUT.webm OUTPUT.webm indexed
cargo run --offline --locked --example remux_audio -- INPUT.webm OUTPUT.webm live 2
```

The example preserves the selected track's supported metadata and complete laced
blocks. It keeps failed staging data and never overwrites an existing destination.
A finalization receipt is distinct from storage durability and intended source
completeness; see the [writing contract](docs/writing.md).

## Existing alternatives

| Library | Choose it for | Current unlace boundary |
| --- | --- | --- |
| [matroska-demuxer](https://github.com/hasenbanck/matroska-demuxer) | Standalone dependency-free Rust demuxing over Read + Seek | Unlace's sequential subset remains under qualification |
| [Symphonia](https://docs.rs/symphonia/latest/symphonia/default/formats/struct.MkvReader.html) | Rust track/packet/seek APIs within an audio framework | Unlace has no codec framework |
| [libwebm](https://github.com/webmproject/libwebm) | C++ parsing/muxing and incremental parser APIs | Unlace is not a qualified replacement |

[The comparison](docs/comparison.md) records further competitors, primary
sources and the mechanisms evaluated. The pinned
[selected-audio comparison](docs/matched-comparison.md) reports exact-field gates,
time, RSS, allocations and fresh-handle seek costs with retained unfavorable
results. It establishes no overall speed or compatibility ranking.

## Evidence and limits

Local tests cover size widths 1–8, unknown-size sentinels, truncation, canonical
IDs and the Matroska 0x80 correction. Exhaustive one/two-byte checks exercise the
integer boundary. Streaming regressions cover all finite-fixture truncations,
short/interrupted reads, continuation at every byte position, unknown-size
termination, malformed boundaries and explicit resource limits. These results do
not qualify complete WebM parsing.

Matched selected-audio measurements and their limits are recorded in the
[comparison report](docs/matched-comparison.md). Compiler source-region
coverage for bounded optional fuzz targets and six-hour completeness evidence are
recorded in [robustness qualification](docs/robustness.md); this is not branch
coverage or a safety proof.
The [qualification protocol](docs/qualification.md) defines equivalent-work
checks required before performance comparisons. The measured consumer job does
not establish interchangeable validation or completion guarantees.

[Encoded WebM writing](docs/writing.md) separates live and indexed
completion, with checked finalization and a staged remux example. Its
[qualification](docs/writer-qualification.md) covers bounded Cluster packing,
six-hour output and mixed/multiple tracks, with a laced-padding playback limitation. Container seeking is
available under the explicit [seek contract and qualification](docs/seeking.md). The sequential reader explicitly
rejects unsupported transformations, CRC-32 and several general Matroska features.
The pinned [MediaRecorder/YouTube corpus](docs/packet-qualification.md) matches
selected packet bytes with two independent demuxers. The
[Opus timing contract](docs/opus-timing.md) includes independently checked
mono/stereo start/end trimming. Broader decoded-audio coverage remains unfinished. It does not decode Opus,
fetch URLs or recognize speech.
The [detailed source audit](docs/source-audit.md) records corrected defects and
remaining qualification risks. The historical [objective audit](docs/objective-audit.md) maps the original requirements to
source, tests and scoped evidence, including the additional
[implementation studies](docs/implementation-studies.md) and
[speech corpus](docs/speech-qualification.md).
The repository is private, package publication is disabled, hosted Actions are
disabled and no release artifact is published. A distribution license has not
been selected. Downloads, docs.rs and CI badges therefore do not apply; ordinary
test-suite branch coverage and general performance superiority remain unproven.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --offline --locked --all-targets -- -D warnings
cargo test --offline --locked
cargo test --offline --locked --release
RUSTDOCFLAGS="-D warnings" cargo doc --offline --locked --no-deps
python3 tools/check_dependencies.py
```

The [changelog](CHANGELOG.md) and [release policy](docs/versioning.md) distinguish
source milestones from published releases. Work is tracked locally in Backlog.md.
