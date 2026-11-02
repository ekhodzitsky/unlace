# Bounded writer fuzz qualification

Assessed on 2026-11-02 against production source `0c4f9ce`, private version
0.2.0. This extends the [reader qualification](robustness.md) with a separate
writer state executable. It does not change production code or qualify arbitrary
Matroska, codec decoding, storage durability or public distribution.

## Target and observable contracts

The optional [runner and input format](../tools/fuzz/README.md) compile directly
with Rust 1.89.0, safe Rust and the standard library. There are no additional
Cargo dependencies, build steps or LLVM requirements for ordinary consumers.
Reader-v1 mode dispatch and existing raw inputs retain their meaning; writer-v1
uses an explicit target selection. Adjacent campaign manifests reject cross-format
replay/reduction. Raw directories without a manifest require correct caller choice.

Every input executes live/indexed output with clean and fault sinks. Mutations
cover configuration, one/two tracks, Opus/Vorbis/video setup, frame slices and
all lacing selectors, signed clocks/trim, group references/flags, packing/direct
output, cue policy, explicit flush/finish, abandonment and terminal reuse.
Controlled sinks inject short writes, Interrupted/WouldBlock/error/zero/invalid
counts, seek errors/wrong positions and flush failures. A repeated-interruption
case must exhaust the configured operation budget.

The target records accepted caller data before inspecting output. A successful
receipt must agree with packet/block/Cluster counts, final byte length and live
or indexed completion. Each enabled audio track is read through End and compared
with caller bytes, raw/presentation clocks, underdetermined later lace times,
flags, default/explicit duration, whole-block and once-only signed padding, and
track initialization. Rejecting a block while remaining Writing must leave output
position, pending bytes and sink call count unchanged. Failed/Finished states must
refuse later write/flush/finish with the same state and no I/O. The target checks
per-operation I/O and pending-buffer allowances.

The output readback uses unlace, so it is not itself an independent implementation
of every format rule. Caller-record comparison prevents a simple matching
writer/reader byte corruption from passing; separate native checks below add
independent evidence for a fixed original seed subset.

## Executed campaigns

Two final-harness campaigns used Rust 1.89.0 (LLVM 20.1.7), compatible
LLVM tools 20.1.5-rust-1.88.0-stable and Python 3.14.4 on Linux x86_64.
Campaign A starts with original writer seeds plus retained pilot inputs; B replays
A's retained corpus with a different RNG seed. Neither found a crash, timeout or
oracle mismatch. There was therefore no production finding to minimize or fix.

| Campaign | RNG seed | Mutations | Seed replays | Executions | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| A | 6110211 | 20,000 | 167 | 20,167 | No failure |
| B | 6110212 | 10,000 | 178 | 10,178 | No failure |

Together these are 30,000 mutations and 30,345 executions. A prior 1,000-mutation
pilot used an earlier harness and is kept separate. The reports replayed 178 and
179 unique retained inputs respectively. Library writer coverage is:

| Campaign | Source | Covered / mapped regions | Lines | Functions |
| --- | --- | ---: | ---: | ---: |
| A | write/ebml.rs | 185 / 200 | 98 / 99 | 16 / 16 |
| A | write/mod.rs | 1385 / 1597 | 859 / 942 | 65 / 71 |
| B | write/ebml.rs | 185 / 200 | 98 / 99 | 16 / 16 |
| B | write/mod.rs | 1385 / 1597 | 859 / 942 | 65 / 71 |

These denominators belong to the instrumented fuzz executable, exclude the
ordinary test suite and are not project-wide branch coverage. The other compiled
reader modules are used for successful-output readback, not assigned a new
general reader qualification score.

Raw manifests, ordered seed hashes, compiler/LLVM pins, profiles, logs and
generated media remain local under ignored project runs directories. No raw
competitor source or operational media is included in the repository.

The tools use LLVM positive source-segment feedback, not branch/edge coverage.
Each input process is limited to 8193 input bytes, 512 MiB address space, two CPU
seconds and three wall seconds, with core dumps disabled. Each sequential worker
has at most 32 actions, eight frames per block, two tracks, a 4096-byte Block,
128 KiB output and 2048-byte index; pending/metadata/frame/lace/I/O bounds vary
within documented small ranges. Parent and LLVM processes have separate costs.
There is no lifetime recording retention in the production writer implied by
this bounded test model; the harness deliberately retains its small oracle.

## Oracle controls and interoperability

Two intentional mutations in isolated local copies of production sources were
both detected with exit 101 using unchanged original seeds:

- Flip the first byte of buffered frame payloads while preserving lengths. The
  caller-record/output comparison fails on the changed byte.
- Remove the Failed transition from Output::fail. A write-error seed exposes
  changed output/call counters while the writer incorrectly remains Writing.

These are test-sensitivity controls, not newly discovered production defects.
The tracked production source was never mutated for these controls.

A manual export of original basic, two-track, negative-clock, reference and all
three lacing/signed-padding seed cases produced 50 completed files. All 54
selected-track jobs reached End with current unlace and pinned classic libwebm;
nonnegative raw-clock jobs also completed Nestegg, totaling 348 native packet
comparisons. Exact payload, normalized presentation time, lace position/count,
whole-block padding, private bytes, delay and preroll agreed. Native whole-block
padding is compared with unlace's explicit block field, without applying it twice.
The negative raw-clock cases use libwebm's raw accessors; Nestegg's unsigned/clamped
time API is not presented as negative-time validation. These are invented encoded
frame bytes, not playable Opus/Vorbis decoding evidence. Upstream pins and adapter
contracts are the same as the [0.2.0 comparison](performance-0.2.md).

A separate seek reader checked 66 forward/backward requests against full
sequential suffixes of the exported indexed basic/two-track/laced fixtures,
including offsets, timing, flags, padding and terminal completion. This adds cue
destination evidence; it is separate from the mutation target's readback.

The default reader target separately replayed 158 unique historical/current seeds
without failure. Both directions of cross-format replay were refused before
output creation. All mandatory Rust 1.89.0 gates pass: formatting, all-target
Clippy with denied warnings, offline locked debug/release tests, warning-free
documentation and the zero-dependency checker (143 integration tests, two unit
tests, five doctests). The writer executable also passes direct denied-warning
compilation and rustfmt; Python sources parse successfully.

## Remaining limits

Coverage does not prove safety or completeness. The small frame/action/output
bounds exclude long histories and all 256-frame combinations; ordinary writer
regressions separately exercise full lace slots and final cue destinations.
Video bytes are receipt-counted but not independently inspected by this target.
Constructor failures consume their sink and do not return an inspectable writer.
Track number/UID/channel/rate fields are mostly fixed; reference counts stop at
three and durations are generated in exact scale ticks. Metadata constructors,
formatting/default helpers, impossible internal bounds, offset/counter overflow
and allocator-error branches are not fully exercised. Allocator failure, all
platform/I/O-provider behavior, arbitrary codecs, real
publication/durability and every state combination remain outside the campaign.

No production fix or API/version change follows from successful fuzzing. The
version stays 0.2.0 and tooling/documentation accumulate under Unreleased. The
[readiness assessment](release-readiness.md) still requires defined consumer
acceptance budgets, representative long workloads and independent review before
a stronger production claim.
