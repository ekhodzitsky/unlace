# Bounded seek qualification for 0.2.2

Assessment: 2026-11-02, Linux x86_64, Rust 1.89.0. This is a private,
unpublished compatible source milestone. The source baseline is 0.2.1 at
`5c22277`; its library and reference executable were preserved before editing.
The MIT grant and disabled publication setting remain in force.

## Change and correctness

Preparation and fallback previously split a valid element's ID/size and Block
prefix into several tiny provider calls. Grouping only the minimum required
prefix bytes reduces calls while preserving bounds and exact validation. Header
storage remains 12 bytes and Block prefix storage 11 bytes. No instance storage,
allocation, dependency, time-order assumption, cached attestation or limit is
added. Fallback still scans the catalogued clusters on every request.

The 128-cluster, three-track regression failed before the change and at the
stronger 1,200-call allowance after Block-only grouping. Both forward/backward
seeks and every resulting packet/End now pass. The existing intentional 50-call
refusal test uses 32 clusters so it still tests refusal and preservation of an
active laced suffix after the call reduction. Four unit tests check every ID/size
width, parent truncation, invalid markers with successful short reads and range
availability retries. Whole-request availability errors can precede malformed
marker errors; retrying still rejects the marker. See the [contract](seeking.md#grouped-index-reads-in-022).

All Rust 1.89 gates pass: formatting, all-target denied-warning Clippy, debug and
release tests (145 integration, six unit, five doctests), denied-warning rustdoc
and the zero-dependency check. All 25 historical selected-input manifest replays
pass. The eight short producer inputs also pass their original four
forward/backward full-suffix queries each (32 total) at the unchanged default
65,536-call allowance. Three fresh complete suffix cohorts cover natural mono and both audio
tracks of the 2.92 GB mixed source, each at 0, 9,000, 19,395 and 0 seconds on one
reader. Every packet field, exact payload, requested one-second interval and
finite completion matches full sequential reading. These are same-library seek
comparisons against sources with [prior native qualification](natural-corpus.md),
not newly independent codec or conformance evidence.

A fresh bounded reader campaign ran 3,000 mutations and 158 seed replays;
a separate writer campaign ran 2,000 mutations and 179 seed replays. Neither
found a failure. All 14 remux outputs retain their prior SHA-256. Three complete
long-input manifests (2,909,420 packets total) retain their full-field hashes
from the independently qualified 0.2.1 inputs; native readers and codec decoding
were not rerun for this index-only change. Retained coverage corpora replayed
successfully and all eight
writer-oracle controls behaved as expected. These small-input campaigns are not
a proof about arbitrary multi-hour hostile inputs or allocation failures.

## Logical reads versus backing traffic

The mixed source is 2,920,934,170 bytes, SHA-256
`4c8a095bc0b9ac29c3036de36bdab30cfa090daa0ba85f907f55ee5a5f19861b`.
It contains video-only Cues and two selected Opus audio alternatives. Source and
producer limits are described in [natural corpus qualification](natural-corpus.md).

The instrumented diagnostic retains the reference 4 KiB provider cache and three
injected availability holes. Diagnostic request counts include a failed hole;
the per-operation ReadReport excludes earlier attempts when the public operation
resets its allowance. Instrumented time is not used as performance evidence.

| Stage | Prepare calls | First audio seek calls | Second audio seek calls |
| --- | ---: | ---: | ---: |
| Preserved 0.2.1 | 155,633 | 271,613 | Refuses at 500,000 |
| Block prefix grouping only | 155,633 | 232,820 | Refuses at 500,000 |
| Header and Block grouping | 116,835 | 155,235 | 367,116 |

With direct buffered SeekSource, full-suffix seek reports are 155,234 calls /
431,887 logical bytes for track 2 and 367,092 / 992,868 for track 3. Cache-edge
short reads explain provider-specific differences. Every request retains the
same 500,000-call and 16 MiB byte budget. The default 65,536-call large-input
control still refuses; no automatic larger retry is introduced.

For the successful 4 KiB-cache first-audio window, backing reads remain 77,860 /
318,910,776 bytes before and after. The newly successful second-audio window
needs 115,727 / 474,014,008 bytes. Baseline track 3 stops early, so its partial
traffic cannot stand in for the cost of a complete baseline seek. Reducing
logical calls does not establish reduced disk or network traffic.

## Measurement method

The refreshed admitted seek matrix contains six queries (two mono, four
six-hour silence) for both unlace revisions, classic libwebm and libavformat;
Nestegg is admitted only for the two mono cases. This is 364 time/RSS trials:
26 jobs, seven shuffled repetitions, warm and advisory-evicted conditions.
Another 78 separate LD_PRELOAD allocation trials cover the same jobs three
times. Input, executable, adapter, runner, supervisor and source hashes are
recorded before execution and checked afterward. The seed is 6110233, affinity
is the first allowed CPU. No other unlace qualification jobs were deliberately
run during measurement, but unrelated active host workload was observed. Its
impact cannot be isolated; no causal attribution or isolated-host claim is made.

The [earlier admission and methodology](performance-0.2.1.md) remain applicable:
64 KiB seek-aware caches; identical selected initialization and one-second
packet/time/trim digest. Actual first packet time must satisfy the source-start-
clamped encoded-preroll bound: `max(source_first_time, target - 80 ms)` for
these jobs. This is not decoder restoration. The digest is a specified subset,
not every Packet field. Pinned competitors are
libwebm `a7c6662c68a90af18dd53dbb1f4262a346e9a547`, Nestegg
`767aab25013acefbdcc6d68a2a7a2c9081303e3f`, and FFmpeg 7.0.2 /
libavformat 61.1.100. No new large-source competitor-seek admission is claimed.
Older sequential/writer comparisons remain historical; this change affects
index I/O and does not require relabeling those numbers as 0.2.2 measurements.

Each process has a 1 GiB address-space cap, 30 CPU seconds and a 60-second owned
process-group timeout. Reference RSS is separately checked against 128 MiB.
These adapter caps do not alter library limits. RSS is process peak KiB and
includes runtime/cache/adapter storage; allocation requests are not retained
heap usage. First-packet timings are adapter internal wall time. Reading the
whole input before warm trials and POSIX_FADV_DONTNEED before advisory trials
is outside timing; eviction is advisory, not guaranteed cold disk. Desktop,
frequency and storage effects remain uncontrolled. No universal speed claim follows.

## Matched seek results

First-packet milliseconds, median [minimum, maximum], seven trials per cell.
Opening, verified seek and required packet delivery are included.


warm

| Source / target seconds | 0.2.1 | 0.2.2 | libwebm | libavformat | Nestegg |
| --- | ---: | ---: | ---: | ---: | ---: |
| mono / 0.0 | 0.059 [0.055, 0.073] | 0.071 [0.055, 0.077] | 0.036 [0.033, 0.040] | 0.076 [0.065, 0.091] | 0.024 [0.019, 0.032] |
| mono / 0.5 | 0.070 [0.062, 0.093] | 0.069 [0.064, 0.090] | 0.039 [0.036, 0.051] | 0.099 [0.080, 0.114] | 0.026 [0.024, 0.030] |
| six-hour / 0.0 | 7.460 [7.355, 7.839] | 6.335 [6.139, 7.012] | 3.634 [3.427, 3.657] | 0.084 [0.080, 0.098] | Not admitted |
| six-hour / 3600.0 | 5.046 [4.417, 5.473] | 4.283 [4.123, 4.739] | 3.693 [3.557, 4.367] | 37.811 [36.772, 46.040] | Not admitted |
| six-hour / 10800.0 | 4.682 [4.207, 5.981] | 4.306 [4.133, 6.812] | 3.778 [3.587, 4.852] | 114.536 [110.865, 147.015] | Not admitted |
| six-hour / 21599.0 | 4.537 [4.412, 5.210] | 4.235 [4.046, 4.896] | 3.619 [3.517, 3.789] | 229.323 [223.309, 234.102] | Not admitted |

advisory-evicted

| Source / target seconds | 0.2.1 | 0.2.2 | libwebm | libavformat | Nestegg |
| --- | ---: | ---: | ---: | ---: | ---: |
| mono / 0.0 | 0.236 [0.233, 3.575] | 0.223 [0.198, 0.254] | 0.214 [0.197, 3.922] | 0.242 [0.225, 0.255] | 0.188 [0.169, 0.310] |
| mono / 0.5 | 0.250 [0.229, 0.282] | 0.239 [0.202, 0.259] | 0.203 [0.196, 0.245] | 0.258 [0.236, 0.427] | 0.187 [0.179, 0.230] |
| six-hour / 0.0 | 18.401 [14.475, 23.807] | 14.426 [13.690, 16.565] | 10.718 [9.754, 14.745] | 0.413 [0.389, 2.203] | Not admitted |
| six-hour / 3600.0 | 12.552 [11.569, 15.558] | 11.800 [11.275, 13.645] | 11.471 [9.841, 13.932] | 41.509 [37.862, 70.961] | Not admitted |
| six-hour / 10800.0 | 14.804 [11.629, 46.333] | 11.930 [11.291, 14.289] | 9.955 [9.461, 14.247] | 119.988 [111.129, 215.407] | Not admitted |
| six-hour / 21599.0 | 12.250 [11.394, 15.305] | 13.502 [11.432, 77.590] | 9.757 [9.319, 11.343] | 239.234 [226.164, 457.503] | Not admitted |

Competitors remain faster on several admitted workloads, while libavformat's
long-file seeks perform substantially more work in this adapter. Small cases
and overlapping ranges do not establish a consistent speedup. No cross-library
leadership claim follows from this scoped matrix.

Allocation request counts are unchanged. Raw requested bytes are 27 lower for
0.2.2 in each job, exactly the executable argument-zero path length difference.
Thirty-six follow-up allocation trials with identical argument zero confirm
identical request and byte counts between revisions in every job. This adapter
path effect is not a library memory improvement. No added index cache or
retained memory pays for the reduced calls.

## First and repeated large seeks

A separate reference variant performs the same 9,000-second seek and one-second
window three times on one reader. Iteration zero includes preparation; later
iterations start after the previous window ends and reuse the prepared reader.
All retain 500,000 calls, the 4 KiB provider cache, bounded retries and injected
range holes. Only measured artificial sleeps are subtracted from active wall
time. The variant records every latency observation without aborting at a
250 ms miss, allowing later iterations to be measured; the unchanged ordinary
reference consumer still returns an error on such a miss. It adds no library
cache or larger library allowance. This diagnostic variant uses the matched
harness's 1 GiB address-space cap; the separate fixed reference cohort below
restores the original 128 MiB cap. There are 56 process trials, seven per track/revision/
cache-condition combination. Each successful process supplies three observations.
The 14 baseline track-3 trials still refuse before a window; their time is not
compared as if they completed equivalent work.

Active first-packet milliseconds, median [minimum, maximum]; misses count
observations exceeding the fixed 250 ms criterion.

| Cache | Track | Revision | First including prepare | Repeated seek | Misses / observations |
| --- | ---: | --- | ---: | ---: | ---: |
| warm | 2 | 0.2.1 | 59.202 [56.025, 82.605] | 27.229 [25.928, 39.294] | 0 / 21 |
| warm | 2 | 0.2.2 | 57.262 [53.679, 79.203] | 31.938 [25.749, 36.513] | 0 / 21 |
| warm | 3 | 0.2.1 | Budget refusal | Budget refusal | No complete observations |
| warm | 3 | 0.2.2 | 81.275 [75.080, 100.308] | 50.994 [45.192, 65.712] | 0 / 21 |
| advisory-evicted | 2 | 0.2.1 | 6637.202 [6201.483, 11659.454] | 39.693 [34.340, 133.576] | 7 / 21 |
| advisory-evicted | 2 | 0.2.2 | 6279.680 [6066.024, 6578.387] | 49.388 [38.756, 64.684] | 7 / 21 |
| advisory-evicted | 3 | 0.2.1 | Budget refusal | Budget refusal | No complete observations |
| advisory-evicted | 3 | 0.2.2 | 11964.293 [11634.210, 13698.503] | 95.922 [89.835, 321.628] | 10 / 21 |

The repeated seek retains a complete cluster scan and similar backing traffic.
A warm OS cache and lower logical-call overhead do not provide a storage/network
latency bound. All cold-start misses are retained even when later iterations
pass. There are
also secondary-track repeated-seek misses in the advisory-evicted cohort. The
first-audio repeated warm median is worse after the change, despite fewer calls;
this experiment establishes improved admission, not a universal latency win.

## Fixed reference pilot follow-up

The same fourteen reference configurations were run three times (42 trials):
six sequential, two paused browser, six ranged. Digests/counts match historical
successful cohorts; newly admitted large track 3 matches its separately checked
window. Source, retry, CPU, wall, address-space, RSS and first-active criteria are
unchanged. The fixed cohort enforces the original 128 MiB address-space cap.
The preceding exploratory cohort used the matched harness's 1 GiB cap, so it is
recorded separately and does not substitute for this fixed-budget evaluation.
Its 42 trials had 39 passes and three large-secondary latency misses
(1,705.330, 398.481 and 302.301 ms); the raw attempts are retained.
This local reference uses ordinary cache state, with no per-trial
conditioning; it is distinct from the controlled large-seek experiment above.

Successful trials: 41 / 42. Failed trials: 1.

- Large secondary-audio ranged request, repetition 1: consumer exits with the
  first-active latency error; 439.164 ms and 9,484 KiB RSS.

The [original pilot](reference-pilot.md) remains historical evidence with call
refusals and a 521.320 ms miss. This change closes the observed 500,000-call
secondary-track refusal on this fixture. Latency/storage-provider policy and an
actual production consumer remain separate qualification requirements. The MIT
choice and these results do not authorize publication or unrestricted production.
Raw media, profiles, immutable pins, harnesses and logs remain local under
ignored runs; this report contains accepted sanitized results only.
