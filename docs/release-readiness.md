# Public release and production readiness

Assessment date: 2026-11-02. Current version: **0.2.2, private and unpublished**.
The original implementation audit covers `0c4f9ce`; the compatible 0.2.1
[reader refactoring](reader-performance.md) adds pinned profiles, repeated corpus
checks and a separate second-agent review. The [source audit](source-audit.md) and
[current measurements](performance-0.2.1.md), [second-agent review](independent-review.md)
and [reference pilot](reference-pilot.md) provide evidence for this assessment.
The [0.2.2 follow-up](seek-performance-0.2.2.md) adds reviewed bounded seek-read
grouping, repeated suffix checks and new fixed-budget measurements.

**Public release: not ready now. Production: no general production approval.**
The historical reference pilot retains its failed criteria. The 0.2.2 follow-up
closes the observed secondary-audio 500,000-call refusal on the same large fixture.
The fixed-budget follow-up passes 41 of 42 trials, with one 439.164 ms secondary-
audio miss against the provisional 250 ms active-wall criterion. Separately,
advisory-evicted large first-packet observations take about 6 to 14 seconds;
several repeated secondary seeks also miss the criterion. The owner has not
selected an actual production consumer/SLA. Passing source tests is not release
or application approval. This assessment does not authorize publication, repository
visibility changes, public tags, release uploads or hosted workflows.

## Public distribution

| Gate | Current evidence | Disposition |
| --- | --- | --- |
| Distribution license | Owner selected MIT; LICENSE contains its terms and Cargo metadata declares MIT. | Licensing gate resolved. Publication remains a separate owner action. |
| Publication authorization | Repository remains private, Cargo publish=false, no public release/tag authorized. | Separate owner action required before distribution. Asking whether it is ready does not authorize publication. |
| Coherent source/API version | Cargo.toml/lock both 0.2.2; changelog and migration notes include accumulated reader/writer incompatibilities. | Passed for a private source milestone. Pre-1.0 status itself is not a reason a clearly experimental alpha could never be released. |
| Package construction | Local offline locked packaging compiles from the generated package. Package-list audit excludes raw runs, Backlog, credentials, target and machine state. | Passed locally; a successful package is not a license or production qualification. |
| Supported scope and documentation | Explicit audio/container subset, ownership, bounded work, error/EOF states, seek suffixes and writer receipts; complete consumer examples. | Documented. Experimental public wording must retain these boundaries. |
| Compatibility and support commitment | Migration notes exist; no stable 1.x, supported-platform matrix or production support/security-response commitment has been established. | Define the intended alpha scope and owner contact/support process before making broader public promises. |
| Public claims | Pinned equivalent-work reports with absolute metrics/ranges and unfavorable cases. | No universal speed, full Matroska conformance or production claim is justified. |

MIT resolves the licensing gate. After the owner authorizes publication and
defines the intended experimental support/communication scope, local build/package
evidence can support evaluating an explicitly experimental release. That conditional path
is different from declaring the current repository publicly released or ready
for unrestricted production use. No crate or repository publication follows from
the license choice.

## Production qualification

| Area | Established evidence | Remaining limit |
| --- | --- | --- |
| Correctness review | Every production module reviewed; two validation defects reproduced before fixes; shared delay/header logic; exact metadata and source/state contracts. | The original audit was author review. A separately authorized second agent later reviewed critical contracts and 0.2.1 buffer reuse, finding three fuzz-oracle gaps that were reproduced and closed. The 0.2.2 grouped-read and regression changes were also reviewed without a confirmed production defect. This is same-session review, not external certification. |
| Mandatory local gates | Rust 1.89 formatting, denied-warning all-target Clippy, debug/release tests, denied-warning rustdoc and zero-dependency check. 145 integration tests, six unit tests, five doctests. | Tests pass on the recorded Linux x86_64 setup, not an asserted platform matrix. |
| Exact encoded audio | Complete real producer/synthetic manifests, all laces, Opus delay/whole-block padding, mono/stereo and added speech decoded-span checks. | Broader multichannel/video decoding and arbitrary codec setup are not qualified; decoding remains outside the library. |
| Long input | Six-hour silence plus [5.39-hour natural speech and a 2.92 GB mixed source](natural-corpus.md); 9,698,068 native packet comparisons and full mono/stereo rolling decoded checks pass. | Concatenated audiobook speech and repeated synthetic video are scoped fixtures, not real conversations or a consumer/platform qualification. Mixed seek exceeds the default before-first budget. |
| Hostile data and state | Every-byte small-fixture truncations/source faults, false offsets, range holes, cancellation and latched revision checks. Final audit campaign: 2,000 mutations plus 156 seed replays without a finding; historical campaigns remain separate. | Bounded short-input campaigns are not a safety proof. The [writer qualification](writer-fuzzing.md) records 30,000 initial and 18,000 reviewed-oracle follow-up mutations; the 0.2.1 refactoring adds 3,000 reader and 2,000 writer mutations, and a separate 0.2.2 campaign repeats 3,000 reader and 2,000 writer mutations plus retained seeds and oracle controls. These scopes remain separate. Exhaustive allocator-fault coverage is still absent. |
| Writing | Exact output hashes, independent readers, every-byte/finalization faults, bounded packing; three long staged pilot readbacks plus no-clobber/truncation controls. | Generic Write + Seek cannot retract externally visible bytes after failed final flush. File sync and no-replace publication do not establish directory crash durability; consumer duties remain. |
| Seeking | Actual structural boundary/cue/track/time checks, starts/preroll/fallback, full suffixes and source identity; simulated range resumes in the pilot. | No decoder restoration/network deadline guarantee. Default large-input requests can refuse. The 0.2.2 fixture passes both audio alternatives within unchanged 500,000-call and 16 MiB seek budgets, with exact repeated full suffixes. Provider-specific latency remains unbounded; no larger default or silent retry is introduced. |
| Resources | Explicit metadata/block/index/depth and per-operation budgets, fallible storage, no lifetime media/result retention in the reader/live writer. | Provider latency, honest immutable revisions, external buffering, allocator internals and application result retention remain consumer contracts. |
| Performance | 1,484 paired time/RSS and 324 allocation trials, strict/payload gates and common-cache/natural/large checks. Warm strict six-hour median is 29.9% lower than preserved 0.2.0. | Other readers remain faster on many jobs; small advisory-evicted cases can regress and large strict ranges overlap. One host, uncontrolled desktop/frequency effects and no application-specific acceptance target. The reader retains 8 KiB extra instance storage from 0.2.1. The 0.2.2 report adds 364 matched seek time/RSS, 78 raw allocation and 36 normalized-argument controls; fewer calls do not consistently lower latency, and several competitors remain faster. |
| Reference consumer | First cohort 39 successful trials and three refusals; supervised cohort 38 successful, three refusals and one 521.320 ms first-active miss. Other observed RSS/wall ceilings pass. The 0.2.2 fixed follow-up passes 41 of 42, with one 439.164 ms secondary miss; its separate advisory-evicted experiment retains 6 to 14 second first-packet times and repeated-seek misses. | A completed evaluation with failed criteria, simulated local providers and a stated digest subset. No actual application rollout, network cache SLA or blanket profile approval. |
| Interoperability | Pinned independent reader and scoped decoder agreement, current packet/writer replay after fixes. | Explicit CRC/transformation/metadata subset; zero-default Opus/browser interpretation and ordinary FFmpeg laced-padding playback disagreement remain visible. |

A small supported format subset can be production-appropriate for a defined
consumer. General Matroska breadth, beating every competitor, and hosted Actions
are not mandatory gates invented by this assessment. The present evidence simply
does not authorize a broad production claim. The completed reference evaluation
provides concrete budgets and negative results. An actual consumer must choose
its input/platform/provider, immutability/durability and latency/resource policy
and test that workload. The reference budgets are not owner-approved application
promises, and their observed latency failures remain unresolved within that envelope. The
fixture-specific call-budget refusal is resolved by 0.2.2 without increasing limits.

## Priorities before a stronger readiness claim

1. Define the intended public experimental support/security-reporting process
   and obtain explicit publication approval. The owner-selected MIT license is
   already recorded.
2. Choose the actual production consumer/input/platform/provider and acceptance
   budgets. Resolve the observed first-latency misses for that workload without
   silently raising limits; the current fixture call-budget refusal is closed. Exercise actual cache
   deadlines and storage failure/durability policy; local simulations do not
   establish them.
3. Retain the completed second-agent review of framing, exact timing, index
   attestation and writer state, including its reproduced oracle corrections.
   Additional external assurance depends on the intended consumer risk profile.
4. Extend the [bounded writer campaigns](writer-fuzzing.md) for the chosen risk
   profile and add allocator-fault testing. Preserve immutable tool/source pins,
   minimized regressions for findings and explicit coverage gaps.
5. Use the completed profiles, compatible 0.2.1 buffer reuse, 0.2.2 grouped reads
   and paired measurements
   as a baseline. Further optimization should follow the actual consumer target
   and preserve all metadata/completeness guarantees and unfavorable evidence.

These are readiness gaps and owner decisions, not claims that every item has been
completed or extra work silently included in this audit. Existing source changes,
measurements, version alignment and the readiness assessment are complete within
the requested scope. The owner-selected MIT grant is recorded; no release,
repository visibility or external service change follows.
