# Bounded concurrent consumer pilot on 0.2.4

Assessment: 2026-11-02, private unpublished source, Linux x86_64, Rust 1.89.0.
The library baseline is `14c624b477134de2a589a4b2fddcbecfa96c86f6`.
This is qualification of a local encoded-packet hash consumer, not an external
application rollout, decoder, network service or general production approval.
Source, Cargo metadata and API remain 0.2.4; this documentation-only evaluation
requires no new source milestone. Raw consumers, media, pins and measurements
remain local under the project evidence policy.

## Fixed profile and ownership

The limits were recorded before control and load execution. At most two source
slots may be occupied, including pending preparation and closing providers.
Each slot retains one original PreparedSource provider and selected audio track.
At most two requests may be admitted, with one per source and no consumer wait
queue. Eight fixed client threads use 256 KiB stacks. Their single-slot command
channels belong to the burst driver, not an admission queue. Busy and capacity
outcomes are explicit refusals. Successful responses require a complete expected
one-second packet window, not only an opened session or first packet.

Source IDs are checked, monotonically issued and never reused. Exhaustion refuses
before opening a provider. They select the live owned generation, not a file path
or an independently matching snapshot tuple. Inputs are pinned immutable files;
the provider asserts a local revision and caches at most 4 KiB. This does not
authenticate arbitrary storage or detect a dishonest revision provider.

Admission reserves capacity and registers cancellation before opening a File.
Ready publication is atomic with the stop check after full preparation. No global
pool lock spans provider I/O, destruction or worker joins. Failed preparation
closes its provider before returning the reservation. An active lease remains
held until the session and provider guard have gone. Eviction keeps the source
slot occupied through actual File destruction; closing work cannot temporarily
create a third live provider. Shutdown cancels registered work, aborts driver
rendezvous, disconnects channels, joins every worker and then drains idle owners.

| Limit | Value and measurement boundary |
| --- | --- |
| First processed packet | 250 ms from consume entry, including admission, opening and full first-packet fingerprinting; also checked from driver dispatch before rendezvous release. |
| Complete window | 1 second from consume entry, checked independently on the final response after lease cleanup. |
| Preparation | 60 seconds per source, checked around bounded prepare calls; at most 10,000 calls. This is cooperative, not a hard end-to-end admission/destruction deadline. Process wall time and external supervision remain separate. |
| Per-operation reads | 500,000 calls and 16 MiB, explicit unchanged library limits. |
| Consumer packet/private/window payload | 64 KiB per packet, 64 KiB codec private data, 1 MiB selected window payload; at most 100,000 polls. |
| Source membership | At most 3 GiB and the pinned fixture within six hours; duration is established by fixture membership, not trusted Info metadata. Request targets outside 0 through six hours refuse. |
| Application process | 128 MiB address space, 128 MiB RSS criterion, 45 CPU seconds and 90 seconds externally supervised wall time. MALLOC_ARENA_MAX=1 is part of this measured Linux profile. Memory-tool processes are separate. |

The preparation bound does not replace the earlier 250 ms unknown-file criterion.
All fourteen advisory-evicted first-from-construction failures in the
[prepared-session evaluation](prepared-sessions-0.2.4.md) remain failures. The
profile requires completed explicit preparation before concurrent service.

## Controlled lifecycle and failure evidence

Barrier-controlled cases exercise capacity including closing, stale IDs and target
bounds, failed preparation rollback, stopping before provider creation and just
before Ready publication, exclusive/global admission, cancellation with pending
laces, transient availability and snapshot I/O, fatal data I/O, latched revision
A-B-A, active shutdown, ID overflow and an intentional provider panic. Thirty-two
evict/open races require either a valid original-source response with eviction
refused, or successful eviction with opening refused. A separate one-active
configuration verifies the global check independently of the per-source limit.
No stale ID routes to a replacement provider. Fatal and poisoned owners remain
unavailable until explicit destruction; recovering a poisoned mutex is limited
to destruction, never resumed packet service.

These cases use a five-second allowance around intentional gates and supply no
250 ms timing evidence. The six-packet laced fixture is an independently built
container control with invented encoded payloads, not decoder qualification.
Its first draft omitted required Info application metadata and was correctly
rejected; the corrected fixture was pinned separately. This was a fixture error,
not a library acceptance change.

Independent review identified a local driver error. A retained local
reproduction showed an early return after partial job dispatch leaving two
detached workers at an uncancellable barrier, with process thread count rising
from one to three and descriptors from four to six. The corrected driver owns all worker handles from construction, uses an
abortable rendezvous and drops result receivers before joining blocked senders.
Five injected failures at partial spawn, partial dispatch, pre-release, receive
and conditioning now restore one thread and four descriptors immediately;
spawned/joined counts are two/two or eight/eight with zero cleanup errors.
This correction is in the local harness; the library source is unchanged.

Independent isolated mutants bypass source capacity, bypass global request
capacity, release a closing reservation early and leak a File while reporting
successful gauge cleanup. Every mutant is detected at its intended assertion;
the descriptor leak is caught by the separate /proc census. Changing only the
packet keyframe flag leaves the historical digest unchanged but fails the new
full-field sequential fingerprint oracle. Clean controls pass. This is scoped
same-session review, not external certification or exhaustive fault injection.

Cooperative shutdown is measured after a controlled provider call is released.
A deliberately blocking provider takes over 350 ms to return after cancellation,
therefore fails 250 ms. A never-returning provider is killed with its owned
process group after two seconds. These negative controls remain outside the
profile and demonstrate that cancellation cannot interrupt arbitrary synchronous
provider methods. No hard cancellation guarantee follows. The cooperative gated shutdown returns
in 0.017 ms after release in this control; that observation does not change the
blocking-provider limitation.

## Golden records and load method

The 2,920,934,170-byte mixed fixture has SHA-256
`4c8a095bc0b9ac29c3036de36bdab30cfa090daa0ba85f907f55ee5a5f19861b`.
It contains the previously qualified natural mono/stereo Opus alternatives and
video-only Cues. Two owners independently prepare tracks 2 and 3, retaining their
original providers. Targets are 0, 1,000, 9,000 and 19,395 seconds. The source,
consumer, driver, profile, small controls and executable are pinned before runs;
after-run checks require identical bytes. No decoded output is produced here.

A fresh ordinary seek reader obtains each actual start. A separate sequential
Demuxer streams the entire selected track and emits complete selected Track,
SegmentInfo, SeekResult and every packet field for the expected window. Python
independently fingerprints those records, including flags, durations, trim and
metadata, using FNV-1a over Rust 1.89 Debug records plus newline. This is a pinned
internal comparison format and a non-cryptographic regression digest, not a
portable serialization or collision proof. Full sequential counts must remain
969,806 and 969,807 with finite completion; the mono and laced controls require
52 and six. These are same-library golden traversals anchored to the historical
native-qualified fixture hashes, not a new independent decoder comparison.

Every successful window must match that full-field fingerprint plus the prior
packet count, byte count, payload/timestamp/whole-padding digest, actual start,
cluster offset and completion. Each nominal window contains 50 selected packets.
Windows normally stop before suffix End, so completion is None; no complete-
suffix validation is claimed for this load cohort. Earlier full-suffix campaigns
remain separate. Errors retain partial progress but cannot return successful
digests. A refused request is counted independently from admitted success.

Each process uses eight persistent clients for 32 bursts of eight attempts.
Warm and advisory-evicted conditions each run seven processes on one allowed CPU
and seven on the first two allowed CPUs. Warmup reads the whole input outside the
application process. Advisory trials request POSIX_FADV_DONTNEED before process
setup and before every burst. Both conditions reset the provider cache before
each burst. Preparation, cache conditioning and process wall time remain visible
and are excluded from the prepared request clock. Advice does not prove physical
cold storage. The host is shared; this is not an isolated throughput benchmark.
A single CPU may serialize otherwise simultaneous clients and therefore admit
more requests than two CPUs. No competitor capacity or speed claim follows;
the existing [equivalent-work comparison](prepared-sessions-0.2.4.md) remains the
separate fresh-reader evidence.

## Results and decision

The preregistered cohort completes all 28 processes and 7,168 attempts:
**3,964 admitted successes and 3,204 explicit Busy refusals**. All successes
match both golden methods. There are zero admitted deadline misses, unexpected
statuses, wrong packet/metadata windows or cleanup invariant failures. Refusals
are not successes and are not hidden from the denominator.

The following percentiles use nearest rank, aggregated over successful requests
in each seven-process group. First-packet columns include dispatch and
rendezvous scheduling; complete-consume time starts at consume entry.

| Condition | CPUs | Success / Busy | First packet median / p95 / p99 / max (ms) | Complete consume max (ms) |
| --- | ---: | ---: | --- | ---: |
| Warm | 1 | 1,792 / 0 | 1.821 / 3.396 / 4.072 / 5.259 | 0.892 |
| Warm | 2 | 1,276 / 516 | 1.212 / 3.146 / 4.181 / 5.507 | 2.998 |
| Advisory-evicted | 1 | 448 / 1,344 | 1.617 / 3.369 / 4.906 / 7.662 | 6.460 |
| Advisory-evicted | 2 | 448 / 1,344 | 1.355 / 1.952 / 2.825 / 3.579 | 3.749 |

The maximum first packet from consume entry is 5.871 ms. Maximum Busy response
cost is 0.023 ms from consume entry and 5.777 ms from driver dispatch. These
small values do not establish throughput or a general tail guarantee. With
advisory-evicted reads only two requests per burst are admitted: six of eight
are deliberately refused. The 44.7% overall rejection rate is an operational
tradeoff of this fixed zero-queue policy, not evidence that all callers meet a
successful-response SLA.

| Condition | CPUs | Two-owner setup median / max (s) | Largest one-source preparation (s) | Whole process max (s) |
| --- | ---: | --- | ---: | ---: |
| Warm | 1 | 0.148 / 0.162 | 0.094 | 0.282 |
| Warm | 2 | 0.148 / 0.153 | 0.088 | 0.243 |
| Advisory-evicted | 1 | 11.447 / 12.691 | 7.319 | 13.060 |
| Advisory-evicted | 2 | 10.938 / 11.396 | 6.156 | 11.737 |

Every process enforces 128 MiB address space and returns to its independent
four-descriptor baseline after shutdown. Each joins all eight workers, reports
zero live source descriptors and zero invariant errors. Peak occupied slots and
source descriptors are two; peak admitted requests is two except warm one-CPU
runs, where it is one. Final RSS is at most 9,328 KiB. This is a quiescent RSS
observation, not a per-trial peak sampler. Separate whole-process peak RSS probes
are recorded separately rather than relabeling final RSS as peak. Two additional
32-burst, two-CPU probes under GNU time observe peak RSS upper bounds of 19,764
KiB warm and 19,744 KiB advisory-evicted, including launcher accounting. They
retain the 128 MiB address-space cap, validate all successful strict fingerprints
and request deadlines, and return 171 successes/85 Busy and 64/192 respectively.
Their 512 attempts are not pooled into the preregistered 7,168; they supply
whole-process memory evidence, not a second latency benchmark. Neither these
observations nor the address-space cap bound kernel page-cache memory or an
application's other processes.

A separate 128-cycle small-source churn repeatedly creates, consumes and evicts
two owners. Descriptors return to four on every cycle; sampled RSS rises from
3,128 to 3,160 KiB and stays there. A separate Valgrind 3.26 Memcheck run of the
same churn reports zero memory errors, zero definitely/indirectly/possibly lost
bytes and 456 bytes still reachable. It runs outside the 128 MiB application
address-space profile because of instrumentation overhead. This does not prove
all allocation failures or arbitrary lifetimes safe.

All six local Rust 1.89 verification gates pass again: formatting, denied-warning
all-target Clippy, debug and release tests, denied-warning rustdoc and dependency
audit. The suite retains 154 integration tests, eight unit tests and eight
doctests, including two compile-fail cases. Frozen source and Cargo hashes agree
before and after evaluation; no normal, build, dev, optional or target-specific
third-party dependency was added. Final same-session evidence review passes
60 aggregate/table/pin/gate checks and individually verifies all 7,168 outcomes.
Offline package construction and verification pass with 83 files, including this
report and excluding local operational evidence and private task records.

**Go for this finite local prepared-consumer profile**, with explicit Busy
handling and completed preparation before serving. **No general production go**:
there is no chosen deployed application/SLA, real provider latency/cancellation
contract, network/backpressure integration, crash recovery, cross-platform
qualification or process-restart persistence. Unknown-file 250 ms acceptance
still fails. A downstream application must establish its own admission/retry,
provider and shutdown requirements; the local success is not that application
rollout. Public distribution still requires an owner decision and a monitored
reporting contact under the [release assessment](release-readiness.md).
