# unlace

Unlace is a pure Rust library for bounded WebM and Matroska container reading
and writing, with no third-party Cargo dependencies and no unsafe code.
The current source milestone is **0.2.4**, private and unpublished, under [MIT](LICENSE).
Rust **1.89.0** is required.

The library preserves encoded audio packets and their timing/trim metadata.
It provides incremental EBML framing, all block lacing modes, sequential audio
demuxing, verified seeking, source-bound prepared sessions and live/indexed WebM
writing from encoded blocks. Decoding, encoding, networking and recognition
belong to consumers. General Matroska features outside the documented subset
can be refused explicitly.

**Readiness:** the [bounded local prepared-consumer pilot](docs/consumer-pilot-0.2.4.md)
passes its admitted-request criteria with explicit Busy refusals. General
production readiness is not established; unknown-file first-use latency still
misses the reference criterion. See the [current release assessment](docs/release-readiness.md)
for the qualified scope and remaining distribution prerequisites.

## Try the source

From this checkout, with no build-time downloads:

```sh
cargo run --offline --locked --example ebml_header
```

Expected output:

```text
id=0x1a45dfa3 size=unknown
```

The integer API reports incomplete input without allocating:

```rust
use unlace::{Parsed, parse_size};

assert_eq!(parse_size(&[0x40]), Ok(Parsed::NeedMore { total: 2 }));
```

An unknown size describes integer syntax, not permission for an element to use
unknown size or for a caller to allocate from an untrusted length.

| Task | Example or guide |
| --- | --- |
| Frame a synthetic EBML stream | `cargo run --offline --locked --example stream_ebml`; [framing contract](docs/streaming.md) |
| Read selected audio | `cargo run --offline --locked --example audio_packets -- INPUT.webm 2`; [demuxing](docs/demuxing.md) |
| Resume short or paused input | `cargo run --offline --locked --example continue_audio -- INPUT.webm`; [continuation](docs/demuxing.md#continuation-example) |
| Seek to presentation nanoseconds | `cargo run --offline --locked --example seek_audio -- INPUT.webm 300000000000 2`; [seeking](docs/seeking.md) |
| Reuse a prepared original provider | `cargo run --offline --locked --example prepared_audio -- INPUT.webm 1 0 500000000 0`; [prepared sessions](docs/seeking.md#prepared-source-sessions-in-024) |
| Remux encoded audio | `cargo run --offline --locked --example remux_audio -- INPUT.webm OUTPUT.webm indexed`; [writing](docs/writing.md) |

File inputs must remain immutable while owned. Preparation is explicit and may
refuse the default limits on large sources; prepared ownership does not persist
an index or import verification from another provider. The remux example uses
staging and refuses to overwrite an existing destination. A finalization receipt
is separate from source completeness and storage durability.

## Documentation and evidence

The [documentation index](docs/README.md) separates current contracts, development
instructions and dated qualification reports. Start with [design](docs/design.md)
for scope and [verification](docs/verification.md) for local checks and their limits.
The [changelog](CHANGELOG.md) and [version policy](docs/versioning.md) distinguish
source milestones from published releases.

The [competitor studies](docs/comparison.md) and
[versioned measurements](docs/README.md#performance-and-competitors) retain exact
pins, equivalent-work gates and unfavorable cases. They establish no general
speed or compatibility ranking. Native packet/trim checks, long inputs, fuzzing
and writer qualification each have a separate stated scope.
