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
use the commands in the [repository README](../README.md#examples) and the
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

## Badge sources

The source, license and Rust badges are local static SVGs describing source
metadata. The CI badge is served by GitHub and links to the actual manual
workflow status for main; it is not a production-readiness certificate.

| Badge | Authoritative source |
| --- | --- |
| Source 0.2.4 | [Cargo.toml](../Cargo.toml) package.version and [Cargo.lock](../Cargo.lock); this badge identifies the source version; registry availability and Git release tags are separate facts. |
| MIT | Cargo license metadata and the actual [LICENSE](../LICENSE) notice. |
| Rust 1.89.0 | Cargo rust-version 1.89 and [rust-toolchain.toml](../rust-toolchain.toml) channel 1.89.0; minimum-version checks run locally. |
| CI | [GitHub Actions CI workflow](https://github.com/ekhodzitsky/unlace/actions/workflows/ci.yml), authorized by the owner on 2026-11-03 and triggered manually. |

Update badges when their authoritative values change. Do not add downloads,
crates.io, docs.rs, coverage or release badges without corresponding published
artifacts and evidence. The [version policy](versioning.md) distinguishes source
milestones, tags and package publication.

## Manual hosted verification

The owner authorized CI on 2026-11-03, replacing the previous disabled-Actions
policy. [ci.yml](../.github/workflows/ci.yml) runs on Ubuntu 24.04 with Rust
1.89.0, the same six required checks above and local source-package verification.
The job has read-only repository contents permission, a 15-minute timeout and a
commit-pinned checkout action without persisted credentials. Toolchain setup
needs network access; Cargo verification remains offline and locked. No cache,
private corpus, model, secret or release artifact is uploaded.

Only workflow_dispatch is configured. To request a check of main:

```sh
gh workflow run ci.yml --ref main
```

The workflow must be on the default branch for manual dispatch, as described in
[GitHub's manual-run guide](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow).
The badge shows the latest workflow status for main and may refer to an earlier
commit until another run completes. Check the run's commit when assessing a
particular revision. Passing source checks does not qualify application latency,
codec output, other platforms or general production use.

On 2026-11-03, [manual run 41645399937](https://github.com/ekhodzitsky/unlace/actions/runs/41645399937)
passed all six required checks and local package verification for commit
`3faed583d4339ea429a74c92fe66167bf5e3551a` on Ubuntu 24.04 / Rust 1.89.0.
This records that exact revision; subsequent documentation commits are not
implicitly included in the result.

## Crates.io publication

The owner-authorized first experimental package, [unlace 0.2.4](https://crates.io/crates/unlace/0.2.4),
was published on 2026-11-04 from commit
`db65b55f950caccf74bd1cc9c0705a20af7838a0`. All six required local Rust 1.89
checks and a clean-checkout Cargo publication dry-run passed. The
[candidate's manual CI run](https://github.com/ekhodzitsky/unlace/actions/runs/41791158372)
also passed every check and local package verification for that exact commit.
Independent review checked the complete 91-file archive, source identity,
metadata and private-file exclusions before publication.

The crates.io version metadata, downloaded archive and local verified candidate
agree on SHA-256:

```text
350514bd4728a4a72a51f189fc20610347a9fee0b5a7205d83477b155ba97e34
```

The archive's Cargo VCS record identifies the source commit above. A separate
Cargo project resolved the exact registry version, built and ran the EBML
integer smoke example on Rust 1.89, then repeated successfully with offline and
locked resolution. Its dependency graph contains only that consumer and unlace;
the library still has zero third-party Cargo dependencies. Local source commits
that only record this receipt are separate from the immutable published package.
The [docs.rs API page](https://docs.rs/unlace/0.2.4/unlace/) was also verified
available after publication. Publication does not change the
[production assessment](release-readiness.md).
