# unlace

Unlace is a private Rust project for streaming WebM and Matroska demuxing.
The **0.1.0 source milestone** provides allocation-free EBML integer parsing.
Unreleased work adds schema-driven incremental framing over `Read`; audio packet
extraction is not available yet.

[Design](docs/design.md) · [Competitors](docs/comparison.md) · [Verification](docs/verification.md)

The target is a small container library with bounded memory, exact packet and
timing information, sequential input without mandatory seeking, and independently
verified seek behavior. Opus decoding belongs to a separate codec library.

## Try the current foundation

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
resource limits. This core does not yet validate a WebM document.

## Existing alternatives

| Library | Choose it for | Current unlace boundary |
| --- | --- | --- |
| [matroska-demuxer](https://github.com/hasenbanck/matroska-demuxer) | Standalone dependency-free Rust demuxing over Read + Seek | Unlace has no demuxer yet |
| [Symphonia](https://docs.rs/symphonia/latest/symphonia/default/formats/struct.MkvReader.html) | Rust track/packet/seek APIs within an audio framework | Unlace has no codec framework |
| [libwebm](https://github.com/webmproject/libwebm) | C++ parsing/muxing and incremental parser APIs | Unlace is a Rust foundation, not a qualified replacement |

[The comparison](docs/comparison.md) records further competitors, primary
sources and the capabilities we intend to evaluate. Superiority is a project
goal; no speed, resource-use or compatibility ranking has been established.

## Evidence and limits

Local tests cover size widths 1–8, unknown-size sentinels, truncation, canonical
IDs and the Matroska 0x80 correction. Exhaustive one/two-byte checks exercise the
integer boundary. Streaming regressions cover all finite-fixture truncations,
short/interrupted reads, continuation at every byte position, unknown-size
termination, malformed boundaries and explicit resource limits. These results do
not qualify complete WebM parsing.

There are no representative container benchmarks or coverage measurements yet.
The [qualification protocol](docs/qualification.md) defines equivalent-work
checks required before performance comparisons. No numerical advantage is claimed.

Unlace does not yet read track metadata, split blocks/lacing, extract packets,
seek or write containers. It does not decode Opus, fetch URLs or recognize speech.
The repository is private, package publication is disabled, hosted Actions are
disabled and no release artifact is published. A distribution license has not
been selected. Downloads, docs.rs and CI badges therefore do not apply; coverage
and end-to-end benchmarks remain unmeasured.

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
