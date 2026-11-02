# Verified fallback reuse in 0.2.3

Assessment: 2026-11-02, private unpublished source, Rust 1.89.0 on Linux x86_64.
The preserved baseline is 0.2.2 at `f0e26e5`. Its source identities, library and
consumer executables were retained before implementation. The earlier
[0.2.2 grouped-read report](seek-performance-0.2.2.md) remains historical.
MIT and disabled publication remain unchanged. There is no general production
or first-use latency approval.

## Mechanism and contracts

A successful complete fallback scan now retains the first selected-track block's
raw ticks/offset, or verified absence, for each catalogued Cluster. Entries stay
in source order: timestamps may be nonmonotonic or tied, and tie selection remains
unchanged. The ready key is the selected TrackNumber; source identity belongs to
the existing immutable RangeReader snapshot. No additional track multiplier,
media lifetime cache or chronological-order assumption is introduced.

Every later fallback still considers the complete catalogue, checks cancellation,
converts exact timestamps and charges the original per-Cluster header work to
max_scan_headers. It performs fresh snapshot checks at operation start, before
returning the candidate, and before committing the packet cursor. Cached lookup
has zero provider reads, so ReadReport contains zero bytes/calls and no fabricated
read extents. Snapshot methods are outside that read-call counter and may perform
provider work. Providers must still bound their own latency and honor immutable,
non-reused revisions; this cache is not source authentication.

Summaries become reusable only after the public seek has completed its final
checks and successfully restarted the packet cursor. Partial/failed scans do not
accumulate usable progress across budget, availability or cancellation failures.
A cache rebuild for another internal track invalidates the prior key before
staging fields. Current public readers keep selection fixed. Chosen Cues always
retain their own track/time/relative-position/CodecState attestation; fallback
summaries never substitute for that work. Every emitted suffix packet is still
read and validated under the supported container contract.

No read, scan, metadata, entry or index-payload limit increases. The first fallback
still requires all original validation and can refuse the default 65,536-call
allowance on the large source. There is no automatic larger retry. Reuse does
not remove preparation or first-fallback cost from the 250 ms criterion.

## Storage tradeoff

On the measured Linux x86_64 target, private Entry layout grows from 48 bytes
(alignment 8) to 112 bytes (alignment 16): 64 bytes extra per entry. At the unchanged
65,536-entry limit this is 4 MiB extra nominal entry storage. Vector capacity and
allocator overhead are additional and target-dependent. All catalogue entries
reserve these fields, including non-Cluster entries and chosen-cue-only readers.
There is no second cache allocation site, but larger existing preparation
allocations can fail earlier under memory pressure. This is not allocation-failure
equivalence or a free memory optimization. The existing max_index_bytes cap still
covers finite Cues/SeekHead payloads; catalogue entries are bounded separately.

Process allocation request counts below are equal between revisions. Requested
bytes include geometric catalogue growth, not just final retained capacity; they
are not retained heap or RSS. Equal-length executable paths avoid the earlier
argument-zero path-length confound. Each allocation job has three identical
observations.

| Input / target seconds | Requests 0.2.2 / 0.2.3 | Requested bytes 0.2.2 / 0.2.3 |
| --- | ---: | ---: |
| mono / 0 | 35 / 35 | 72,471 / 73,239 |
| mono / 0.5 | 37 / 37 | 74,015 / 74,783 |
| six-hour / 0 | 65 / 65 | 8,564,491 / 12,758,539 |
| six-hour / 3600 | 64 / 64 | 8,548,119 / 12,742,167 |
| six-hour / 10800 | 64 / 64 | 8,548,120 / 12,742,168 |
| six-hour / 21599 | 64 / 64 | 8,548,120 / 12,742,168 |

## Phase I/O qualification

The unchanged 2,920,934,170-byte mixed source, SHA-256
`4c8a095bc0b9ac29c3036de36bdab30cfa090daa0ba85f907f55ee5a5f19861b`,
has video-only Cues and two selected audio alternatives. Its producer and
natural-speech scope remain in [natural corpus qualification](natural-corpus.md).
The reference provider uses a 4 KiB seek-aware cache and immutable file revision.
A diagnostic records provider-cache refills, separately from logical RangeReader
requests. Refills request 4 KiB where available, including bytes the parser does
not inspect. These are provider backing-file calls/returned bytes, not measured
physical disk sectors or network traffic. Diagnostic instrumentation time is not
used as benchmark evidence.

| Phase | 0.2.2 backing calls / bytes | 0.2.3 backing calls / bytes |
| --- | ---: | ---: |
| Prepare, either track | 38,995 / 159,719,736 | 38,995 / 159,719,736 |
| First seek, track 2 | 38,792 / 158,892,032 | 38,792 / 158,892,032 |
| First seek, track 3 | 76,659 / 313,995,264 | 76,659 / 313,995,264 |
| Each repeated seek, either track | Same as its first seek | 0 / 0 |
| Each one-second packet window, either track | 73 / 299,008 | 73 / 299,008 |

First-seek logical requests remain 155,234 calls / 431,887 bytes for track 2 and
367,115 / 992,868 for track 3 with this provider. Every successful new repeated
seek reports zero logical calls/bytes. Direct buffered SeekSource first-track-3
calls differ slightly (367,092) due to provider short reads/cache edges. The
unchanged explicit allowance is 500,000 calls and 16 MiB per operation.

## Correctness and independent review

The repeated-source-read regression was recorded failing before implementation;
both reuse regressions also fail against the preserved 0.2.2 library. Ordinary
checks cover unknown/finite Clusters, unsorted and tied times, verified track
absence, forward/backward suffixes, sixteen consecutive 50-call refusals with
pending laced output, late cancellation and outer snapshot I/O failure, latched
revision A-B-A and forged chosen-cue time/relative offsets after fallback reuse.
Private tests retain original work charges including absence and invalidate a
track key before a partial rebuild. An independent copied-source early-commit
mutant is detected by the public late-failure regression. The authorized same-
session reviewer found no production defect; this is not external certification.

All mandatory Rust 1.89 gates pass: formatting, denied-warning all-target Clippy,
debug/release tests (149 integration, eight unit, five doctests), denied-warning
rustdoc and zero third-party dependencies. All 25 historical manifest cases,
32 short-input default-budget full-suffix queries and 12 long explicit-budget
queries pass. The long queries cover natural mono and both large audio tracks,
0/9,000/19,395/0 seconds on one reader, every packet field and exact payload,
requested windows and final completion. Chosen-cue queries still perform I/O.
Three complete long manifests (2,909,420 packets) and all 14 remux hashes match
prior independently qualified inputs/outputs. Native readers/PCM decoding were
not rerun: reuse of that evidence depends on exact complete-manifest/output
identity, not a new independent conformance claim.

Fresh bounded campaigns ran 4,000 reader mutations plus 159 seed replays and
2,000 writer mutations plus 179 seed replays without a finding. Retained coverage
corpora replayed (164 reader, 179 writer) and all eight writer-oracle controls
behaved as expected. Small bounded campaigns do not prove arbitrary hostile
long-input safety, allocator-fault behavior or platform compatibility.

## Measurement method

The admitted fresh-handle seek matrix is unchanged: six queries (two mono,
four six-hour silence) for both unlace revisions, classic libwebm and
libavformat; Nestegg is admitted only for the two mono cases. There are 364
time/RSS trials: 26 jobs, seven shuffled repetitions under warm and advisory-
evicted input conditions. Another 78 separate LD_PRELOAD allocation trials
cover the same jobs three times. These readers use a common 64 KiB seek-aware
cache and the established selected initialization/one-second packet/time/trim
digest. First packet times must satisfy the source-start-clamped encoded-preroll
bound, max(source_first_time, target minus 80 ms) for these jobs. This digest is a
stated subset, not every Packet field or decoder restoration.

Pinned competitors remain libwebm `a7c6662c68a90af18dd53dbb1f4262a346e9a547`,
Nestegg `767aab25013acefbdcc6d68a2a7a2c9081303e3f`, FFmpeg 7.0.2 /
libavformat 61.1.100. Prior [admission details](performance-0.2.1.md) apply.
No fresh large-source competitor-seek or same-reader repeated-competitor
admission is claimed. Comparing a reused unlace reader to a newly opened
competitor would not be equivalent work. Unaffected sequential/writer speed
results remain historical, not relabeled as 0.2.3 measurements.

A separate 4 KiB-cache reference variant performs the 9,000-second seek and
one-second window three times on one reader. Iteration zero includes preparation;
iterations one/two reuse the prepared reader after the previous window ends.
Both revisions retain 128 MiB address space, 30 CPU seconds, 60 wall seconds,
500,000 calls, 16 MiB read bytes and the same 250 ms first-active criterion.
The process logs every observation, continuing after a latency miss to measure
later iterations. Only measured artificial waits are subtracted from active wall
time. On reused fallback no source read occurs to consume a seek-stage range
hole: the injected hole is encountered during packet polling instead. First
iterations exercise three holes; later 0.2.2 iterations exercise two each,
0.2.3 iterations one each. No dummy I/O was added to preserve old fault counts.
Source/cancellation/snapshot error behavior is also covered by dedicated tests.

There are 56 large repeated process trials, seven per track/revision/cache
combination, each with three observations. The matched matrix uses a separate
1 GiB address-space cap; its wall/CPU limits are the same. A corrected owned-
process-group supervisor bounds each process tree. Input, executable, adapter,
source, runner and supervisor hashes are recorded before runs and checked after.
Seed is 6110243, affinity the first allowed CPU. No other unlace qualification
jobs were deliberately run during timing. This is a shared host; background
load, desktop, storage and frequency effects are uncontrolled. Warm input reads
and POSIX_FADV_DONTNEED requests occur outside timing; eviction is advisory,
not proof of cold disk/cache state. RSS is whole-process peak KiB. No universal
latency or superiority claim follows.

## Fresh-handle competitor comparison

First packet milliseconds, median [minimum, maximum], seven trials per cell.
Opening, verified seeking and required packet delivery are included. This table
measures first use, where the new cache is not yet reusable.


warm

| Source / target seconds | 0.2.2 | 0.2.3 | libwebm | libavformat | Nestegg |
| --- | ---: | ---: | ---: | ---: | ---: |
| mono / 0 | 0.051 [0.048, 0.063] | 0.050 [0.049, 0.061] | 0.034 [0.028, 0.039] | 0.063 [0.058, 0.128] | 0.021 [0.020, 0.032] |
| mono / 0.5 | 0.059 [0.056, 0.066] | 0.063 [0.058, 0.080] | 0.035 [0.033, 0.036] | 0.075 [0.073, 0.100] | 0.022 [0.020, 0.031] |
| six-hour / 0 | 5.717 [5.599, 5.836] | 6.132 [5.826, 6.417] | 3.265 [3.195, 3.558] | 0.072 [0.067, 0.077] | Not admitted |
| six-hour / 3600 | 3.817 [3.770, 4.061] | 4.212 [4.009, 4.388] | 3.331 [3.243, 3.407] | 32.791 [32.040, 34.001] | Not admitted |
| six-hour / 10800 | 3.871 [3.764, 3.982] | 4.067 [3.987, 4.282] | 3.405 [3.273, 3.548] | 99.550 [95.673, 106.587] | Not admitted |
| six-hour / 21599 | 3.855 [3.718, 4.369] | 4.120 [3.929, 4.337] | 3.274 [3.228, 3.570] | 192.674 [192.301, 203.124] | Not admitted |

advisory-evicted

| Source / target seconds | 0.2.2 | 0.2.3 | libwebm | libavformat | Nestegg |
| --- | ---: | ---: | ---: | ---: | ---: |
| mono / 0 | 0.223 [0.205, 0.247] | 0.208 [0.199, 0.226] | 0.200 [0.191, 0.234] | 0.224 [0.195, 0.251] | 0.170 [0.162, 0.219] |
| mono / 0.5 | 0.216 [0.211, 0.373] | 0.222 [0.213, 0.243] | 0.193 [0.189, 0.199] | 0.227 [0.219, 0.243] | 0.184 [0.170, 0.208] |
| six-hour / 0 | 13.187 [12.696, 14.407] | 13.196 [12.772, 14.600] | 9.243 [9.014, 10.383] | 0.473 [0.394, 0.504] | Not admitted |
| six-hour / 3600 | 11.202 [11.051, 11.574] | 11.426 [11.022, 16.219] | 9.323 [9.106, 9.426] | 34.825 [32.850, 37.510] | Not admitted |
| six-hour / 10800 | 11.098 [10.970, 11.618] | 11.418 [11.086, 88.048] | 9.404 [9.194, 10.318] | 102.733 [99.407, 120.938] | Not admitted |
| six-hour / 21599 | 11.085 [10.900, 69.746] | 11.688 [11.085, 12.908] | 9.569 [9.167, 10.394] | 207.355 [198.619, 222.685] | Not admitted |

Larger catalogue entries can make first-use work slower and consume more memory;
this optimization targets reuse, not universal first-use speed. Competitors
remain faster on several admitted jobs. Broad overlaps and small submillisecond
cases do not establish a consistent improvement.

## First and repeated large reference requests

Active first-packet milliseconds, median [minimum, maximum]. First includes
preparation; repeated cells pool both later iterations (14 observations).
Misses count observations exceeding the unchanged 250 ms criterion. Later
iterations request the same position/window already read by iteration zero;
these timings do not qualify arbitrary cold destination windows. Summary reuse
removes prefix I/O, while requested packet reads remain provider-dependent.

| Cache | Track | Revision | First including prepare | Repeated | Misses / observations |
| --- | ---: | --- | ---: | ---: | ---: |
| warm | 2 | 0.2.2 | 64.145 [61.822, 70.130] | 29.896 [28.149, 43.200] | 0 / 21 |
| warm | 2 | 0.2.3 | 68.595 [64.060, 95.671] | 0.263 [0.237, 0.449] | 0 / 21 |
| warm | 3 | 0.2.2 | 90.175 [84.407, 95.366] | 54.797 [50.525, 63.500] | 0 / 21 |
| warm | 3 | 0.2.3 | 85.483 [85.000, 93.628] | 0.262 [0.228, 0.388] | 0 / 21 |
| advisory-evicted | 2 | 0.2.2 | 5724.100 [5691.485, 5836.078] | 30.018 [27.780, 32.211] | 7 / 21 |
| advisory-evicted | 2 | 0.2.3 | 5754.261 [5692.494, 5824.111] | 0.265 [0.237, 0.392] | 7 / 21 |
| advisory-evicted | 3 | 0.2.2 | 10997.312 [10894.412, 35065.173] | 53.063 [50.138, 77.728] | 7 / 21 |
| advisory-evicted | 3 | 0.2.3 | 11004.991 [10887.629, 11809.569] | 0.279 [0.233, 0.511] | 7 / 21 |

Whole-process peak RSS KiB across the large reference processes.

| Revision | Minimum | Maximum |
| --- | ---: | ---: |
| 0.2.2 | 9,396 | 9,612 |
| 0.2.3 | 11,832 | 12,112 |

## Fixed reference profile and readiness

The same fourteen configurations (six sequential, two paused browser, six
ranged) were run three times, 42 trials. They enforce the original 128 MiB
address-space and RSS criteria, 30 CPU/60 wall seconds and 250 ms first-active
criterion. Digests/counts match historical qualified results; the large second
track matches its separately checked window. Each fresh ranged trial still
exercises the original three availability holes. No per-trial cache conditioning
is applied in this cohort, distinct from the controlled large-seek comparison.

Passed 42 / 42; failed 0.
All observed criteria pass in this cohort; that is not a worst-case latency bound or erasure of historical failures.

First preparation/fallback is still expensive after advisory eviction. Successful
reuse does not repair those failures or approve arbitrary network/storage
providers. The [experimental scope](experimental-scope.md) defines the supported
subset, compatibility and current private reporting process; a monitored public
contact and publication authorization remain owner prerequisites. The
[readiness assessment](release-readiness.md) retains the actual-consumer and
first-use latency gaps. All raw corpora, adapters, profiles and pins stay local
under ignored runs. No crate, repository visibility or release artifact is
published by this source milestone.
