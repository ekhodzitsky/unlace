# Verification

Use the pinned Rust 1.89.0 toolchain from the repository root. Ordinary checks
need no network, third-party Cargo dependencies, external codecs or downloaded
media. Current source and API are 0.2.4; source versions do not imply publication
or production qualification.

## Required local checks

```sh
cargo fmt --all -- --check
cargo clippy --offline --locked --all-targets -- -D warnings
cargo test --offline --locked
cargo test --offline --locked --release
RUSTDOCFLAGS="-D warnings" cargo doc --offline --locked --no-deps
python3 tools/check_dependencies.py
```

`rust-toolchain.toml` pins 1.89.0; use `cargo +1.89.0` explicitly when overriding
a local toolchain. Clippy covers all examples and test targets. The dependency
check inspects locked offline Cargo metadata and rejects normal, build,
development, optional and target-specific third-party dependencies.

The 0.2.4 suite has eight unit tests, 154 integration tests and eight doctests,
including two compile-fail checks for prepared-session ownership. These are
suite counts, not a coverage percentage. Synthetic fixtures exercise exact
bytes, timestamps/trim, lacing, EOF/completeness, bounds, source faults,
continuation, seek transactions and writer failure states. They do not establish
arbitrary WebM conformance or decoder correctness.

## Examples and package hygiene

The self-contained framing examples are smoke checks without media:

```sh
cargo run --offline --locked --example ebml_header
cargo run --offline --locked --example stream_ebml
```

For file-based audio, continuation, seek, prepared sessions and remux examples,
use the commands in the [repository README](../README.md#try-the-source) and the
corresponding [contracts](README.md#current-contracts). A file-based example
requires a suitable local input and may refuse unsupported structure or resource
limits. A successful window is not evidence of a fully validated source; a
writer receipt is not proof of input completeness or durable publication.

Before an authorized source commit, check package contents locally:

```sh
cargo package --offline --locked --list
cargo package --offline --locked
```

Use `--allow-dirty` only to verify an intentional uncommitted change. Packaging
constructs and verifies a local archive; it does not publish. Inspect the file
list against the [local-only rules](../agents/local-only-files.md). Raw `runs/`,
Backlog records, credentials and generated output must remain excluded.

## Evidence boundaries

The [documentation index](README.md#qualification-and-audits) maps separate
native packet/trim checks, long inputs, output readbacks, audits and bounded
reader/writer campaigns. The [fuzz-tool guide](../tools/fuzz/README.md) describes
optional instrumented runs, pins, process limits and replay/oracle controls.
Those tools are separate from ordinary Cargo verification. Source-region fuzz
coverage is not branch coverage, and a finite campaign is not a safety proof.

The [qualification protocol](qualification.md) requires equivalent work before
performance comparisons. Preserve failed admissions, first-use latency, cache
conditions, memory costs and digest/completion scope. Do not combine different
campaigns or relabel prepared-query latency as unknown-source startup.

Verification so far qualifies Linux x86_64 with Rust 1.89.0. It does not establish
Windows/macOS/ARM parity, arbitrary allocation-failure recovery, bounded blocking
provider calls or a deployed application's SLA. The [release assessment](release-readiness.md)
is the authority for current readiness. The [historical verification notes](history/verification-milestones.md)
preserve earlier suite counts and unfinished-work statements through 0.2.1.
