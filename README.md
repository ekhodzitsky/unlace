# unlace

Bounded WebM and Matroska containers in pure Rust.

[![Source version: 0.2.4](docs/assets/version.svg)](Cargo.toml)
[![License: MIT](docs/assets/license.svg)](LICENSE)
[![CI](https://github.com/ekhodzitsky/unlace/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ekhodzitsky/unlace/actions/workflows/ci.yml)
[![Minimum Rust: 1.89.0](docs/assets/msrv.svg)](rust-toolchain.toml)

Read encoded audio packets with their original bytes, timestamps and trim
metadata. Continue sequential input, seek through verified container positions,
or write WebM from already encoded blocks. Unlace has no third-party Cargo
dependencies and forbids unsafe Rust. Decoding and encoding belong to your codec.

## Examples

Run these examples from a [source checkout](#use-from-source) with a suitable
local WebM input. They use the library and expose its contracts; they are not
installed commands or codec implementations.

**Read selected audio packets.** Use the first enabled audio track, or give an
explicit TrackNumber:

```sh
cargo run --offline --locked --example audio_packets -- INPUT.webm
cargo run --offline --locked --example audio_packets -- INPUT.webm 2
```

The manifest includes original packet bytes, timing, lacing and whole-block
padding, followed by explicit observed completion. Packets borrow the reader's
storage until its next poll. [Audio contract](docs/demuxing.md).

**Continue paused input.** This example resumes the same owned reader across
short reads and `WouldBlock`:

```sh
cargo run --offline --locked --example continue_audio -- INPUT.webm
```

It does not import a saved checkpoint onto another source or resume after a
process restart. [Continuation](docs/demuxing.md#continuation-example).

**Seek to a passage.** Requests use signed presentation nanoseconds; this one
requests 300 seconds on track 2:

```sh
cargo run --offline --locked --example seek_audio -- INPUT.webm 300000000000 2
```

The reader reports the actual access point, required preroll, fallback and
suffix completion. A container access point does not restore decoder state.
[Source identity](docs/sources.md) and [seeking](docs/seeking.md).

**Reuse a prepared source.** Retain the original immutable provider across
exclusive sessions at 0, 0.5 and 0 seconds:

```sh
cargo run --offline --locked --example prepared_audio -- INPUT.webm 1 0 500000000 0
```

Preparation is explicit and may refuse resource limits on large files. It does
not persist a disk index or transfer verification to an independent provider.
[Prepared sessions](docs/seeking.md#prepared-source-sessions-in-024).

**Remux encoded audio.** Write indexed or live WebM without re-encoding:

```sh
cargo run --offline --locked --example remux_audio -- INPUT.webm OUTPUT-indexed.webm indexed
cargo run --offline --locked --example remux_audio -- INPUT.webm OUTPUT-live.webm live
```

The example stages output and refuses to overwrite an existing destination.
Checked finalization is separate from source completeness and storage durability.
[Writing contract](docs/writing.md).

## Built for container consumers

- **Bounded work and storage.** Explicit limits cover headers, nesting, metadata,
  blocks, indexes and read operations. Refusal remains visible when a limit is hit.
- **Packet fidelity.** Preserve encoded bytes, track identity, lacing, signed
  timestamps and trim metadata. Unknown timing stays unknown.
- **Explicit state.** Distinguish incomplete input, unsupported structure,
  malformed data, resource refusal, I/O failure and observed completion.
- **Verified reuse.** Attest seek destinations and retain source checks across
  exclusive prepared sessions. Consumers own immutable revisions and admission.
- **Encoded-block writing.** Separate live output from indexed finalization;
  expose failure states and checked receipts.

[Design](docs/design.md) | [All contracts and API guides](docs/README.md#current-contracts)

## Use from source

Source version **0.2.4** is public under MIT. No crate, Git release tag or binary
release is published. Rust **1.89.0** is the pinned source toolchain. Once it is
available, ordinary builds and tests need no codec library, model or download.

```sh
git clone https://github.com/ekhodzitsky/unlace.git
cd unlace
cargo test --offline --locked
cargo doc --offline --locked --no-deps
```

A consuming Cargo project beside the checkout can use a path dependency:

```toml
[dependencies]
unlace = { path = "../unlace" }
```

For a self-contained smoke check without media:

```sh
cargo run --offline --locked --example ebml_header
```

It prints `id=0x1a45dfa3 size=unknown`. Unknown-size integer syntax does not
validate a containing document or authorize allocation from an untrusted length.
[Local verification and badge sources](docs/verification.md).

## Compared with

The pinned studies evaluate different container APIs and validation guarantees:

| Evaluated project | Role in the recorded comparison |
| --- | --- |
| matroska-demuxer | Standalone Rust demuxing over Read + Seek |
| Symphonia | Rust audio framework with track, packet and seek APIs |
| libwebm | C++ parsing and muxing, with classic and incremental parsers evaluated separately |

[Competitor selection](docs/comparison.md), [implementation studies](docs/implementation-studies.md)
and [versioned measurements](docs/README.md#performance-and-competitors) record
exact revisions, equivalent-work gates, cache conditions, time, RSS, allocations
and unfavorable cases. These dated observations establish no overall speed,
compatibility or current-market ranking.

## Evidence

| Evaluation | Result and boundary |
| --- | --- |
| [Packet and timing qualification](docs/packet-qualification.md) | Scoped native-reader agreement; [Opus trimming](docs/opus-timing.md) and [long inputs](docs/natural-corpus.md) have separate methods. |
| [Bounded concurrent prepared consumer](docs/consumer-pilot-0.2.4.md) | 3,964 matching admitted windows and 3,204 Busy refusals across 7,168 attempts. Go only for that fixed local prepared profile. |
| [GigaSTT integration](docs/gigastt-integration.md) | Shared packet and same-decoder crop checks pass; admission, trim and failure behavior prevent a transparent replacement approval. |
| [Packet-level PCM follow-up](docs/gigastt-integration.md#packet-level-localization) | A 53-byte case reproduces a codec-output difference before container transforms; it is not an unlace parser or codec-conformance verdict. |

## Scope

**Experimental container library.** The documented subset includes selected-audio
reading and encoded Opus/Vorbis/VP8/VP9 writing. General Matroska features can be
refused explicitly. Unlace does not decode codecs, fetch URLs or recognize speech.
Source honesty, bounded provider latency and application buffering remain
consumer responsibilities.

General production readiness is not established. Cold preparation of a new
seekable source still misses the reference 250 ms criterion; prepared-query and
sequential decode observations have different boundaries. A monitored public
reporting contact and crate-publication authorization remain outstanding.
[Current readiness, compatibility and reporting](docs/release-readiness.md).

[Documentation](docs/README.md) | [Development checks](docs/verification.md) |
[Changelog](CHANGELOG.md) | [Source version policy](docs/versioning.md) | [MIT](LICENSE)
