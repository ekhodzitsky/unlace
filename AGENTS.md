agent-dev-kb: 0.10.1

# unlace

Core source: agent-dev-kb revision cb223cb, version 0.10.1. The stamp certifies
core instruction content only, not optional MCP setup.

Public source repository. Never change visibility, publish a crate, or upload release
artifacts without explicit owner authorization. Keep all code and project records
in English. Use plain wording without decorative arrows or symbolic shorthand.

## Product direction

Build a pure Rust, zero-third-party-dependency WebM/Matroska container library.
Start with bounded streaming demuxing and audio packet fidelity. Codec decoding,
YouTube access, networking and recognition belong to consumers. Do not claim
superiority without equivalent-work measurements and published methodology.
Never silently relax parsing, trim, timestamp or completeness guarantees.

## Development

Use main. Run checks locally. The owner authorized manual GitHub Actions source
verification on 2026-11-03. Keep hosted checks manually dispatched; release
uploads and scheduled/automatic triggers are not authorized. The owner authorized
the first experimental crates.io publication of 0.2.4 on 2026-11-04. Future
package versions and other releases require their own owner authorization.
Source commits and pushes within the authorized project setup are allowed;
future changes follow the owner instruction for that task. Use Conventional Commits.
Keep Cargo versions, changelog, examples and actual API consistent. Add a failing
regression before correcting tested behavior. No production unwrap/expect or unsafe.
Test-only unwraps may be allowed locally in test modules.
Use Rust 1.89.0 for the minimum-version checks. All normal, build, development,
optional and target-specific third-party dependencies are prohibited.

## Backlog.md

This repo uses Backlog.md (not GitHub Issues, not a second spec tool).
Run `backlog instructions overview` before work. Mutate via the CLI
(`--plain` / `--json`). One task has one agreed scope; sessions may resume it.
Local research: `doc create`; draft conclusions: `decision create`;
work: `task create`. Accepted, sanitized decisions belong in tracked
project docs; local records link to them. Labels: `audit`, `research`.
Keep remoteOperations and autoCommit false.

## Local-only files

Read [local-only rules](agents/local-only-files.md) before creating or staging
artifacts. Backlog, raw competitor research, benchmarks, operational media,
credentials and machine-specific state remain local regardless of repository visibility.

## Verification

cargo fmt --all -- --check
cargo clippy --offline --locked --all-targets -- -D warnings
cargo test --offline --locked
cargo test --offline --locked --release
RUSTDOCFLAGS="-D warnings" cargo doc --offline --locked --no-deps
python3 tools/check_dependencies.py

## Coding principles

[Coding principles](agents/coding-principles.md) — read before acting.
