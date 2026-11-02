# Source and release policy

Unlace remains private. Cargo publish is disabled. Creating a source commit is
not permission to publish a crate, change visibility or upload release binaries.
No GitHub Actions should be configured or run until explicitly requested.

Use SemVer. Before 1.0, incompatible public API changes increment the minor
version; compatible changes increment the patch version when a source milestone
is cut. Accumulate related work under Unreleased instead of cutting a release
for every commit. Keep Cargo.toml, Cargo.lock, documentation and examples aligned.
The current private source milestone is 0.2.1, with compatible reader buffer
reuse and expanded qualification. The earlier 0.2.0 includes the accumulated
incompatible reader/writer API additions after the initial integer-only 0.1.0.
All remain unpublished. Do not tag or advertise a public release without a
deliberate owner decision. A source version does not establish production readiness.

English Conventional Commit messages describe the delivered change. Backlog
identifiers, raw research, private operational data and generated output stay
out of commits even while the remote is private. The owner selected the MIT
license, recorded in LICENSE and Cargo metadata.
That license choice does not authorize repository or package publication.
