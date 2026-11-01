# Source and release policy

Unlace remains private. Cargo publish is disabled. Creating a source commit is
not permission to publish a crate, change visibility or upload release binaries.
No GitHub Actions should be configured or run until explicitly requested.

Use SemVer. Before 1.0, incompatible public API changes increment the minor
version; compatible changes increment the patch version when a source milestone
is cut. Accumulate related work under Unreleased instead of cutting a release
for every commit. Keep Cargo.toml, Cargo.lock, documentation and examples aligned.
The initial 0.1.0 is an unpublished source milestone. Do not tag or advertise a
published release without a deliberate release decision.

English Conventional Commit messages describe the delivered change. Backlog
identifiers, raw research, private operational data and generated output stay
out of commits even while the remote is private. A distribution license must
be selected by the owner before publication; no license grant is invented here.
