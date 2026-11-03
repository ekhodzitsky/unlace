# Experimental distribution scope

This document defines the intended experimental source scope. The repository
remains private, Cargo publishing is disabled, and no public release, tag or
artifact upload is authorized by this policy. MIT is recorded in LICENSE.

## Supported source scope

The library provides pure Rust, zero-third-party-dependency EBML framing,
bounded selected-audio container demuxing, verified container seeking and
live/indexed WebM writing from encoded blocks. The contracts in
[streaming](streaming.md), [demuxing](demuxing.md), [seeking](seeking.md) and
[writing](writing.md) define the supported subset and explicit refusals.
Codec decoding, networking, URL access, recognition and video decoding belong
to consumers. A legal Matroska file can use features outside this subset.
Unsupported CRC/transformation/index features are not silently accepted.

Current verification is on Linux x86_64 with Rust 1.89.0. That is the tested
configuration, not a claim of equivalent qualification on every Rust target.
The [qualification protocol](qualification.md) and versioned reports state the
producer, input, packet, timing, completion and performance evidence. Synthetic
and concatenated long sources do not establish arbitrary application coverage.
An experimental release must preserve these limitations in its description.

## Compatibility and reports

The [version policy](versioning.md) defines pre-1.0 SemVer and source milestones.
There is no stable 1.x API or long-term support promise. Notable changes and
migration requirements belong in CHANGELOG.md and the affected contracts. A
patch version does not establish production or performance approval.

Current collaborators report problems directly to the repository owner through
an existing private channel. Include the source commit, toolchain/platform,
limits, provider/revision behavior, expected and observed result, and a small
original reproducer where possible. Do not send credentials or private media
unless the owner has arranged a suitable private exchange. Accepted work is
tracked in the project's local Backlog workflow; hosted workflows and GitHub
Issues are not an alternate project tracker.

Potential security issues should use an existing private owner channel, with
reproduction details and affected versions. Do not disclose sensitive inputs in
public reports. No monitored public security address, response deadline or
production support SLA is established by this document. Before public
publication, the owner must name a monitored reporting contact and communicate
its scope. This remains an explicit distribution prerequisite.

## Production boundary

Consumers own immutable source identity, bounded provider latency, decoding,
output publication/durability and application acceptance criteria. In particular,
retaining verified seek summaries accelerates reuse; it does not bound first
opening or first fallback latency. The reference 250 ms criterion and its failed
observations remain visible in the [readiness assessment](release-readiness.md).
A consumer must qualify its actual workload and failure policy. Neither MIT nor
a successful package construction substitutes for that evaluation.

Prepared-session qualification is limited to a continuously owned immutable
provider, explicit complete preparation, fixed selection and exclusive consumers.
It is an in-process lifecycle, not disk-persistent verification or independent
file reopening. The [0.2.4 reference evaluation](prepared-sessions-0.2.4.md) passes
its measured prepared-opening criterion on varied windows after advisory cache
resets; first-new-source cost remains seconds. The [bounded local consumer pilot](consumer-pilot-0.2.4.md) adopts that policy
and passes its declared profile. Actual applications must qualify their own
provider, workload, admission and shutdown requirements.
