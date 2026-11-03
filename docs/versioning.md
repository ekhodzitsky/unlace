# Source and release policy

The owner authorized public source visibility on 2026-11-03. Cargo publish
remains disabled. Creating a source commit is not permission to publish a crate,
change visibility again or upload release binaries.
No GitHub Actions should be configured or run until explicitly requested.

Use SemVer. Before 1.0, incompatible public API changes increment the minor
version; compatible changes increment the patch version when a source milestone
is cut. Accumulate related work under Unreleased instead of cutting a release
for every commit. Keep Cargo.toml, Cargo.lock, documentation and examples aligned.
The current source milestone is 0.2.4. The [changelog](../CHANGELOG.md)
records earlier milestones and migration details; none is a published crate release.
Documentation and qualification-only work accumulate under Unreleased without
an automatic source-version bump. Do not tag or advertise a public release
without a deliberate owner decision. A source version does not establish
production readiness.

English Conventional Commit messages describe the delivered change. Backlog
identifiers, raw research, private operational data and generated output stay
out of commits and packages, including public Git history. The owner selected the MIT
license, recorded in LICENSE and Cargo metadata.
Repository visibility is now public by separate owner authorization. The license
choice does not authorize package publication.

As of 2026-11-03, local and remote Git release tags and GitHub Releases are empty.
The 0.2.4 source version and dated changelog entry do not imply a v0.2.4 tag.
A metadata or tag audit does not authorize creating a release; keep publication
separate from source commits and the owner-authorized public repository visibility.
