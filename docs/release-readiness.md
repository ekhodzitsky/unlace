# Public release and production readiness

Assessment date: 2026-11-02. Current version: **0.2.0, private and unpublished**.
The reviewed implementation is `0c4f9ce`; subsequent performance/report changes
are documentation and local research. The [source audit](source-audit.md) and
[current measurements](performance-0.2.md) provide evidence for this assessment.

**Public release: not ready now. Production: no general production approval.**
The completed original implementation scope and a passing test suite are not
release approval. This assessment does not authorize publication, repository
visibility changes, public tags, release uploads or hosted workflows.

## Public distribution

| Gate | Current evidence | Disposition |
| --- | --- | --- |
| Distribution license | No owner-selected license, license file or Cargo license metadata. Cargo package warns about the missing field. | Blocking owner decision under the repository policy; do not invent a grant. |
| Publication authorization | Repository remains private, Cargo publish=false, no public release/tag authorized. | Separate owner action required before distribution. Asking whether it is ready does not authorize publication. |
| Coherent source/API version | Cargo.toml/lock both 0.2.0; changelog and migration notes include accumulated reader/writer incompatibilities. | Passed for a private source milestone. Pre-1.0 status itself is not a reason a clearly experimental alpha could never be released. |
| Package construction | Local offline locked packaging compiles from the generated package. Package-list audit excludes raw runs, Backlog, credentials, target and machine state. | Passed locally; a successful package is not a license or production qualification. |
| Supported scope and documentation | Explicit audio/container subset, ownership, bounded work, error/EOF states, seek suffixes and writer receipts; complete consumer examples. | Documented. Experimental public wording must retain these boundaries. |
| Compatibility and support commitment | Migration notes exist; no stable 1.x, supported-platform matrix or production support/security-response commitment has been established. | Define the intended alpha scope and owner contact/support process before making broader public promises. |
| Public claims | Pinned equivalent-work reports with absolute metrics/ranges and unfavorable cases. | No universal speed, full Matroska conformance or production claim is justified. |

After the owner resolves licensing, publication authorization and the intended
experimental support/communication scope, local build/package evidence can
support evaluating an explicitly experimental release. That conditional path
is different from declaring the current repository publicly released or ready
for unrestricted production use. No license choice is made in this change.

## Production qualification

| Area | Established evidence | Remaining limit |
| --- | --- | --- |
| Correctness review | Every production module reviewed; two validation defects reproduced before fixes; shared delay/header logic; exact metadata and source/state contracts. | Internal review only, not independent review or certification. The earlier passing suite did not prevent finding these defects. |
| Mandatory local gates | Rust 1.89 formatting, denied-warning all-target Clippy, debug/release tests, denied-warning rustdoc and zero-dependency check. 143 integration tests, two unit tests, five doctests. | Tests pass on the recorded Linux x86_64 setup, not an asserted platform matrix. |
| Exact encoded audio | Complete real producer/synthetic manifests, all laces, Opus delay/whole-block padding, mono/stereo and added speech decoded-span checks. | Broader multichannel/video decoding and arbitrary codec setup are not qualified; decoding remains outside the library. |
| Long input | All 1,080,001 packets of a six-hour encoded-silence container matched; bounded result processing. | Repeated silence and a 10.4 MB file do not establish behavior for diverse natural multi-hour or multi-gigabyte content. |
| Hostile data and state | Every-byte small-fixture truncations/source faults, false offsets, range holes, cancellation and latched revision checks. Final audit campaign: 2,000 mutations plus 156 seed replays without a finding; historical campaigns remain separate. | Bounded short-input campaigns are not a safety proof. No new writer coverage-guided mutation campaign or exhaustive allocator-fault campaign is claimed. |
| Writing | Exact current output hashes, independent readers, generated modes/laces/multiple tracks, every-byte writes, finalization seeks/flush faults, bounded packing and staged publication. | Generic Write + Seek cannot retract externally visible bytes after a failed final flush. Applications must honor receipt/staging/durability duties. |
| Seeking | Actual structural boundary/cue/track/time checks and reported starts/preroll/fallback; suffix and source identity checks. | No decoder restoration guarantee. Default before-first-cue budget refuses the six-hour source; explicit larger qualification budget is not a changed default. |
| Resources | Explicit metadata/block/index/depth and per-operation budgets, fallible storage, no lifetime media/result retention in the reader/live writer. | Provider latency, honest immutable revisions, external buffering, allocator internals and application result retention remain consumer contracts. |
| Performance | 952 time/RSS and 210 allocation trials, full strict/payload gates and common-cache checks; low long-file allocation counts. | One host and controlled fixtures, uncontrolled desktop/frequency effects, no application-specific throughput/latency/memory acceptance target. Lower allocation count is not faster iteration. |
| Interoperability | Pinned independent reader and scoped decoder agreement, current packet/writer replay after fixes. | Explicit CRC/transformation/metadata subset; zero-default Opus/browser interpretation and ordinary FFmpeg laced-padding playback disagreement remain visible. |

A small supported format subset can be production-appropriate for a defined
consumer. General Matroska breadth, beating every competitor, and hosted Actions
are not mandatory gates invented by this assessment. The present evidence simply
does not authorize a broad production claim. A controlled integration/pilot must
choose its accepted input profile, immutability/durability policy and resource/
latency budgets, and test its actual workload against those contracts.

## Priorities before a stronger readiness claim

1. Owner resolves the distribution license and the intended public experimental
   support/security-reporting process; publication still needs explicit approval.
2. Define supported production input/platform profiles and consumer acceptance
   budgets. Exercise representative natural long recordings, actual ranged/cache
   providers and real output failure/publication policies on those profiles.
3. Obtain an independent review of framing, exact timing, index attestation and
   writer preflight/finalization state. Track findings and regressions in Backlog.
4. Extend coverage-guided writer/state and allocator-fault testing with bounded
   campaigns, immutable tool/source pins, retained minimized regressions and
   explicit coverage gaps. Increase campaign scope for the chosen risk profile.
5. Profile any throughput bottleneck against the consumer target before optimizing;
   rerun exact equivalence and matched measurements after changes. Preserve all
   current metadata/completeness guarantees and unfavorable results.

These are readiness gaps and owner decisions, not claims that every item has been
completed or extra work silently included in this audit. Existing source changes,
measurements, version alignment and the readiness assessment are complete within
the requested scope. No release, license grant or external service change follows.
