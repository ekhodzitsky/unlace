# Scoped second-agent review

On 2026-11-02 the owner explicitly authorized a separate agent in this session
to review unlace. The reviewer had read-only scope over critical container
contracts, test tooling, the reader refactor, performance evidence and the
reference pilot. It created only local isolated reproductions and records;
accepted sanitized findings and closures are recorded here. This is separate
from the implementation author's self-review, but it is not an external human
security audit, certification or proof of general production readiness.

## Production scope and result

The review covered framing/inherited finite boundaries and lookahead; late
BlockGroup metadata; exact i128 clocks/Opus delay and whole-block trim; source
identity/count/EOF latches; cue boundary/track/time/relative destinations; deferred
packet publication during cancellation; and writer preflight/partial-I/O,
packed/direct output and finalization state. No reproducible production defect
was found against the documented supported contracts. Eighty-five targeted
Rust 1.89 tests passed during the initial review.

A later read-only pass reviewed the compatible 0.2.1 buffer refactor at source
`79a1ebe`, including stale range/scratch contents, partial reads, cancellation,
restart and retained memory. A targeted 256-frame/empty/unselected-block regression
passes both versions; it is promoted into the normal suite. All 21 seek tests
pass against the new library. Per-instance retained size increases by exactly
8,192 bytes on the measured host, as documented in [reader profiling](reader-performance.md).
No concrete defect was found in this change.

## Reproduced tooling findings and closure

| Finding | Missed-before evidence | Correction and closure |
| --- | --- | --- |
| False refusal | An isolated mutant refusing every valid block passed all 157 original writer seeds because expected records depended on Ok. | A guaranteed-valid lane independently requires three accepted blocks, known packet counts and successful finish in both modes. The portable control fails before the enhancement and detects the mutant afterward. |
| Missing successful terminal state | Removing the live Finished assignment still passed all seeds after the acceptance lane. | Every successful explicit/automatic finish must set Finished before terminal no-I/O reuse is checked. Both writer modes have mutation controls. |
| Wrong failure terminal state | Setting Finished instead of Failed for I/O error passed all seeds despite terminal reuse checks. | Typed I/O/position/I/O-budget errors require Failed; invalid/unsupported preflight errors require Writing; state errors must match the actual state. Wrong-state and poisoned-preflight controls are detected. |

The final portable writer control passes clean code and detects all seven
intentional mutants at their intended assertions. New bounded campaigns record
each harness revision separately in [writer qualification](writer-fuzzing.md).
These were fuzz-oracle false negatives, not demonstrated production bugs; ordinary
writer tests and independent fixed-output checks supplied separate protection.

During the local pilot, review also identified wrapper timeout cleanup: killing
GNU time alone can leave its child. The corrected local supervisor kills its
owned POSIX process group; a controlled before/after reproduction confirms closure.
All original observed trials had already finished within their wall ceilings.
This tooling correction does not change library APIs or guarantee provider latency.

## Evidence and claims review

The reviewer checked 282 performance values/counts against the retained jobs and
summary, plus historical/natural/writer/paused/seek qualification counts. One
inconsistent Callgrind summary/footer convention differed by two instructions
and was corrected. No unsupported superiority or validation claim was found in
the [paired performance report](performance-0.2.1.md). Small regressions, broad
ranges, adapter/cache effects and the retained memory cost remain visible.

The [reference pilot](reference-pilot.md) preserves both cohorts and its failed
criteria: three secondary-audio call refusals per cohort and one later large-file
first-latency miss. Pilot digests cover a stated field subset; ranged windows
normally do not reach suffix End. Consumer/input hashes and retrospective wrapper
pins are distinguished. Staged-output readbacks, no-clobber/truncation controls
and the absence of directory crash-durability evidence are explicit.

No further confirmed production issue remained from this scoped review. It does
not replace a consumer's integration, provider/storage failure policy, actual
acceptance budgets, platform qualification or owner distribution decisions.

## 0.2.2 grouped seek reads

The same authorized reviewer separately inspected the bounded header and Block
prefix grouping, the unchanged RangeReader/cancellation path and the strengthened
refusal/admission regressions. Isolated width, truncation and short-read checks
were reproduced before promotion into ordinary unit tests. Review found no
malformed-data acceptance, source/cancellation guarantee weakening or limit
increase. Whole-request availability may precede malformed-marker detection;
the documented retry test retains eventual rejection. No tracked source files
were edited by the reviewer. This remains same-session scoped review.

The reviewer also checked 108 measurement values/counts against the 0.2.2 raw
records, including all reported medians/ranges/misses, allocation controls,
364 matched trials, 56 large repeated processes and both later reference
cohorts. The exploratory 1 GiB and fixed 128 MiB address-space allowances remain
distinct. Review requested advisory-evicted wording where physical cold-cache
state was not established; the readiness text was corrected. No unsupported
superiority or large-source competitor admission claim was found.


## 0.2.3 fallback summary reuse

The authorized same-session reviewer inspected the inline summary design and
transaction boundary. Independent copied-source tests confirm original work
charges including verified absence, key invalidation before a partial different-
track rebuild and no substitution for chosen-cue attestation. A mutation that
commits readiness inside Index::locate before outer checks is detected by the
public late-failure regression. Those work/key tests were promoted into the
ordinary unit suite. Review found no production defect. Entry layout increases
from 48 to 112 bytes on the measured target; memory pressure can cause earlier
preparation allocation refusal. This tradeoff is explicit in the versioned report.

The final report was checked against 111 measurement/count values with no
numerical discrepancy. Phase refills, zero repeat logical reads, dynamic
availability-hole counts, active clocks, suffix/manifest/output counts and both
address-space caps agree with raw records. The report explicitly limits repeat
timings to the same previously read packet window, retaining first-use failures,
chosen-cue-only memory overhead and the absence of network/cold-destination
latency qualification.


## 0.2.4 source-bound prepared sessions

The same authorized reviewer inspected original-provider ownership, exclusive
session lifetimes, atomic positioned openings, one-operation preparation phases
and persistent source/fatal latches. Three isolated mutants are detected: a
hidden catalogue-to-warmup reset, skipped full warmup and readiness committed
before a failed warmup. Additional edge checks cover maximum preroll/delay,
negative timestamps, interrupted short reads, range holes and Ready faults.
No production defect or required correction remained from this review.

The final report passed 184 automated numeric/table/count/layout/source-pin
checks. It preserves 112/112 prepared query successes alongside fourteen initial-
opening misses, distinguishes the existing retained-reader baseline, includes
preparation/conditioning wall cost correctly and separates 128 MiB reference
from 1 GiB fresh-matrix caps. It explicitly limits reuse to the original live
provider, without disk persistence, independent file reopening or general
production/network latency approval. No tracked files were edited by the reviewer.


## Bounded concurrent consumer on frozen 0.2.4

The same authorized reviewer inspected reservation/publication, original-provider
ownership, active leases, closing capacity, cancellation and lock order in the
local consumer. It identified a driver-only partial-dispatch cleanup gap. A
retained local reproduction showed two detached barrier waiters and their
descriptors. The revised abortable rendezvous and RAII worker group join every handle before draining providers;
five injected orchestration failures return thread and descriptor baselines.
No library defect or tracked source change was required.

Four isolated mutants are rejected: source capacity bypass, independent global
request-cap bypass, early closing-capacity release and leaked File with false
cleanup gauges. A keyframe-only packet mutant preserves the legacy digest but
fails an independent sequential full-field fingerprint. These controls establish
specific oracle sensitivity, not exhaustive concurrency or allocator proof.
Blocking synchronous provider methods still require external supervision.
The [consumer report](consumer-pilot-0.2.4.md) retains those limitations, explicit
Busy outcomes, preparation cost and the narrower local-profile decision.

Final evidence review passes 60 aggregate, table, pin and gate checks and verifies
all 7,168 main-cohort outcomes individually. It confirms the 3,964 matching
successes, 3,204 explicit refusals, request/preparation limits, joins and descriptor
cleanup. Separate peak-RSS probes are not pooled into that cohort. Two prose
corrections distinguish dispatch-inclusive first-packet from consume-only
completion clocks and attribute the driver reproduction accurately. No further
required change remains from this scoped review.


## GigaSTT integration pilot

The same authorized reviewer checked the frozen actual consumer, existing
Opus trim/soft-EOF behavior and the isolated adapter. Review found local oracle
scope gaps for gain, successful native completion, amplitude comparison and
immutable input/runtime pins; the corrected cohorts preserve the earlier records.
Four isolated trim tests and a known-passing numeric control with three rejected
corruptions pass. Final evidence review passes 63 aggregate/table/source/pin
checks and individually checks all 252 timing rows, including their correctness
outputs. The [integration report](gigastt-integration.md) retains the failed
numeric PCM gate, two admission differences and the limited 14-word natural
recognition smoke check. No production library defect or transparent-replacement
approval follows. No tracked files were edited by the reviewer.
