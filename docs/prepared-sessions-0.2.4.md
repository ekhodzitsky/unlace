# Source-bound prepared sessions in 0.2.4

Assessment: 2026-11-02, private unpublished source, Linux x86_64, Rust 1.89.0.
The preserved source baseline is 0.2.3 at `5d2c136`. MIT and disabled publication
remain unchanged. This evaluation qualifies a local reference profile, not an
actual application rollout or general production readiness. A subsequent
[bounded concurrent consumer pilot](consumer-pilot-0.2.4.md) exercises the
unchanged source under explicit lifecycle, load and cleanup limits.

## Decision and scope

Snapshot length/revision fields are provider-local assertions. Equal tuples from
independent providers cannot authenticate index transfer. The chosen design
retains the original provider, metadata, catalogue and fallback summaries inside
an opaque PreparedSource. Exclusive consumer sessions borrow that owner. There
is no source replacement, global snapshot-key registry, serialized index,
independent file-handle import or process-restart persistence. The original
provider, including any owned File, remains alive between sessions. An opening
here means a consumer session, not reopening a filesystem path.

This is a typed lifecycle over the existing 0.2.3 retained SeekReader machinery;
it introduces no faster index algorithm. A consumer retaining an ordinary
SeekReader could already reuse verification. The comparison therefore includes
that baseline separately from a fresh reader. Persisting/importing verification
across independent providers requires a stronger identity/trust protocol and
bounded deserialization; it is not silently enabled by matching revision values.

Provider revisions must still be immutable and never reused for changed bytes.
Retaining the original provider establishes provenance within that contract,
not cryptographic authentication or detection of dishonest snapshots. An
application registry must select the correct owned source generation; matching
path names or provider-local tuples alone are insufficient. Owner counts,
lifetimes and eviction are application duties, with no implicit global cache.

## API and bounded preparation

PreparedSource::new owns the provider and captures its snapshot without payload
reads. prepare first advances ordinary metadata/catalogue preparation and always
yields after that stage reaches Ready. A separate call warms the complete
selected-track fallback through the existing seek transaction. Its internal
request has a representable effective preroll target of i128::MIN, before any
supported container timestamp; no packets are consumed. Each public prepare
call starts exactly one existing bounded operation. Read reports do not hide a
second internal reset or omit completed I/O.

Ready is stronger than ordinary SeekReader metadata/index readiness: the full
fallback may hit scan/read limits, return NoAccessPoint for empty selected audio,
or discover prefix errors that a chosen-cue-only request would skip. Chosen-cue-
only sources also pay this preparation cost. Partial or failed scans do not
accumulate a usable fallback cache. Limits and default refusals remain unchanged;
the reference uses explicit 500,000 calls/16 MiB per operation, not larger defaults.

open(request, cancel) returns a SeekSession only after the ordinary seek and
its final identity/cancellation checks succeed. The session exposes its actual
start, immutable metadata, read report and packet polling. No failed opening can
expose the prior consumer's pending laced packets. Dropping a session performs no
I/O and preserves preparation plus fatal/revision latches. Source recovery
consumes the owner and discards verification. Compile-fail doctests enforce
exclusive sessions and prevent provider recovery during an active borrowed session.

Ready attests the catalogue and completed selected-track fallback summaries;
it does not attest every unused Cue, skipped packet or decoder state. Chosen
Cues retain their own track/time/relative/CodecState checks on opening. Every
emitted supported suffix packet is still read and checked to explicit completion.
Source methods can block; bounded method latency remains a provider duty.

## Verification and independent review

The lifecycle regression was recorded failing to compile before the new API.
Five new integration cases cover independent equal-snapshot sources with different
metadata/bytes, dropped/reopened full suffixes, pending laces, cancellation,
snapshot I/O and latched revision A-B-A, fatal Cue failure across turnover,
actual provider-call/report equality and sixteen full-scan refusals without
session readiness. Existing parser, source, timing and transaction tests remain.

All Rust 1.89 gates pass: formatting, denied-warning all-target Clippy,
debug/release tests (154 integration, eight unit, eight doctests including two
compile-fail cases), denied-warning rustdoc and zero third-party dependencies.
The new prepared_audio example completes three finite-source sessions in its
smoke check and reports preparation separately. It uses default limits and can
explicitly refuse a large full scan; it never silently increases the allowance.

The authorized same-session reviewer found no production defect. Three isolated
mutants are detected: removing the catalogue-stage yield (63 provider calls
under a 50-call allowance while reporting 18), skipping full warmup, and marking
Ready before a failed warmup. Independent edge checks cover maximum preroll/delay,
negative timestamps, interrupted short reads, availability holes and Ready faults.
This is scoped same-session review, not external certification.

Reader fuzz mode 3 now exercises both ordinary seeking and prepared-source
session turnover over whole and paused/short positional input. Each lifecycle's
traces must match across schedules; when both complete, their successful traces
must also agree. Error phases need not match the stronger prepared lifecycle.
A fresh campaign ran 4,000 mutations plus 164 seeds without a finding, with
165 retained inputs replayed for coverage. A separate writer campaign ran 2,000
mutations plus 179 seeds; its 179-input coverage replay and all eight writer-
oracle controls pass. These bounded campaigns are not arbitrary hostile-input,
allocator-fault or platform proofs.

All 25 historical manifest replays pass. Prepared sessions pass 32 short default-
budget and 12 long explicit-budget complete-suffix queries. Long cohorts cover
natural mono and both large mixed audio tracks at 0/9,000/19,395/0 seconds, with
every packet field/payload, requested windows and final completion equal to full
sequential iteration. Three full long manifests (2,909,420 packets) and all
14 remux output hashes retain prior independent qualification identities. Native
readers/PCM decoding were not rerun for this lifecycle-only change; reuse of their
evidence depends on exact complete-manifest/output identity.

## Reference measurement protocol

The source is the unchanged 2,920,934,170-byte mixed file, SHA-256
`4c8a095bc0b9ac29c3036de36bdab30cfa090daa0ba85f907f55ee5a5f19861b`.
Its concatenated speech/repeated-video scope remains in
[natural corpus qualification](natural-corpus.md). Each process owns one original
4 KiB-cache positional file provider, one selected audio track and one cursor.
The provider assumes the pinned file remains immutable; this is not qualification
of a real network service or independent file identity mechanism.

Three modes separate the work:

- Fresh SeekReader 0.2.3 prepares metadata and delivers its first 9,000-second
  window naturally, without post-setup cache advice.
- Retained SeekReader 0.2.3 explicitly completes the same full fallback warmup
  before four queries, retaining the original reader/provider.
- PreparedSource 0.2.4 completes its explicit preparation, then opens/drops four
  exclusive consumer sessions on the original provider.

The latter two request 9,000, 19,395, 1,000 and 0 seconds, each once per process.
These windows were not previously consumed by the setup or an earlier query.
Prefix warmup can nevertheless read their file pages. In advisory-evicted trials,
a Python helper advises POSIX_FADV_DONTNEED after setup and before every retained/
prepared query; the original provider's 4 KiB cache is also reset outside the
query clock. Advice is not proof of physically cold pages/disks. Warm trials
read the complete source before starting the process; provider cache is still
reset between queries. Conditioners do not replace the provider or source bytes.

There are 84 process trials: two tracks, three modes, two initial cache
conditions, seven shuffled repetitions, producing 252 windows. Every process has
128 MiB address-space/RSS criteria, 30 CPU seconds and a 60-second owned-process-
group wall timeout. Conditioning helpers inherit these limits; wall supervision
covers the process tree. Process RSS can include the helper's peak and is not
retained library heap. Query latency includes open/seek, identity checks and
packet delivery; no synthetic waits are subtracted. Setup is reported separately.
First-from-construction time includes setup and, for retained/prepared modes,
post-setup conditioning wall time. This differs from an opening on an already
prepared source and preserves the unknown-file first-use cost.

The fixed 250 ms criterion is evaluated for both first-from-construction and
per-query observations, without removing preparation from the former. Existing
read/scan/storage bounds apply to every operation; no automatic larger retry or
hidden reset is used. The one-second digest includes selected CodecPrivate,
CodecDelay/SeekPreRoll and packet bytes/length/presentation time/whole-block trim,
not every Packet field. Complete suffix checks above supply separate full-field
and End evidence. Window completion normally stops before suffix End.

Executables, source, adapter, runner, supervisor, expected output and input hashes
are recorded before timing and checked afterward. The seed is 6110253, affinity
the first allowed CPU. No other unlace qualification work was deliberately run
during timing. Shared-host load, desktop/frequency/storage effects remain
uncontrolled. Twelve separate allocation trials cover retained/prepared modes
three times per track, without cache-advice children; equal-length modes/executable
paths avoid argument-zero size confounds. Results describe these local profiles,
not worst-case deadlines or general speed superiority.


## Reference results

Milliseconds, median [minimum, maximum]. Seven trials per setup cell;
28 observations per query cell pool the four distinct windows.

| Initial condition | Track | Mode | Setup | First from construction | Query first packet | Query misses / observations |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| warm | 2 | Fresh 0.2.3 | 33.811 [32.987, 34.947] | 63.940 [62.370, 65.002] | 29.631 [29.375, 30.678] | 0 / 7 |
| warm | 2 | Retained 0.2.3 | 63.853 [62.642, 69.565] | 64.142 [62.947, 69.858] | 0.247 [0.199, 0.381] | 0 / 28 |
| warm | 2 | Prepared 0.2.4 | 62.701 [61.949, 71.325] | 63.092 [62.232, 71.681] | 0.250 [0.200, 0.624] | 0 / 28 |
| warm | 3 | Fresh 0.2.3 | 36.325 [33.271, 37.488] | 92.017 [84.739, 98.115] | 54.810 [51.240, 61.784] | 0 / 7 |
| warm | 3 | Retained 0.2.3 | 84.629 [82.976, 94.519] | 84.918 [83.265, 94.831] | 0.246 [0.198, 0.301] | 0 / 28 |
| warm | 3 | Prepared 0.2.4 | 84.931 [83.224, 97.656] | 85.240 [83.527, 98.088] | 0.254 [0.199, 0.427] | 0 / 28 |
| advisory-evicted | 2 | Fresh 0.2.3 | 5755.379 [5697.236, 5977.483] | 5807.806 [5747.517, 6033.540] | 50.270 [48.336, 56.047] | 0 / 7 |
| advisory-evicted | 2 | Retained 0.2.3 | 5774.851 [5628.707, 6079.924] | 5806.545 [5657.322, 6124.625] | 1.246 [0.953, 3.116] | 0 / 28 |
| advisory-evicted | 2 | Prepared 0.2.4 | 5813.773 [5698.843, 5876.009] | 5845.634 [5727.761, 5907.204] | 1.193 [0.977, 2.823] | 0 / 28 |
| advisory-evicted | 3 | Fresh 0.2.3 | 5698.599 [5616.306, 5841.436] | 10976.723 [10957.235, 11197.143] | 5282.567 [5258.625, 5451.532] | 7 / 7 |
| advisory-evicted | 3 | Retained 0.2.3 | 11008.369 [10938.244, 11748.300] | 11050.183 [10982.418, 11796.591] | 1.177 [0.972, 2.478] | 0 / 28 |
| advisory-evicted | 3 | Prepared 0.2.4 | 11047.362 [10950.142, 11839.887] | 11097.617 [10993.579, 11882.399] | 1.201 [0.990, 4.401] | 0 / 28 |

All 112 prepared query windows pass the fixed 250 ms criterion. All
14 advisory-evicted prepared first-from-construction observations miss it;
14 of 28 prepared first-from-construction observations miss overall.
The initial cost is not waived or relabeled as a fast unknown-file opening.

The retained 0.2.3 reader already provides similar delivery times. These results
support the new API's explicit ownership/session lifecycle, not a new algorithmic
speedup over retaining that reader. First-new-source and already-prepared-session
work are different jobs and must not be ranked as if interchangeable.

Prepared 0.2.4 per-destination first-packet milliseconds, seven trials per cell.

| Condition | Track | Target seconds | First packet |
| --- | ---: | ---: | ---: |
| warm | 2 | 9000 | 0.290 [0.274, 0.624] |
| warm | 2 | 19395 | 0.252 [0.240, 0.337] |
| warm | 2 | 1000 | 0.247 [0.224, 0.291] |
| warm | 2 | 0 | 0.213 [0.200, 0.267] |
| warm | 3 | 9000 | 0.297 [0.286, 0.415] |
| warm | 3 | 19395 | 0.255 [0.242, 0.427] |
| warm | 3 | 1000 | 0.236 [0.225, 0.290] |
| warm | 3 | 0 | 0.211 [0.199, 0.265] |
| advisory-evicted | 2 | 9000 | 1.481 [1.456, 2.823] |
| advisory-evicted | 2 | 19395 | 1.212 [1.180, 1.265] |
| advisory-evicted | 2 | 1000 | 1.098 [1.057, 1.111] |
| advisory-evicted | 2 | 0 | 1.021 [0.977, 1.243] |
| advisory-evicted | 3 | 9000 | 1.571 [1.457, 4.401] |
| advisory-evicted | 3 | 19395 | 1.280 [1.239, 1.499] |
| advisory-evicted | 3 | 1000 | 1.084 [1.063, 1.163] |
| advisory-evicted | 3 | 0 | 1.017 [0.990, 1.146] |

Each retained/prepared opening reports zero logical/provider backing reads for
prefix lookup, even after advisory eviction. Subsequent packet delivery performs
normal I/O. Preparation backing traffic is unchanged from the retained baseline:
38,995 reads / 159,719,736 bytes for the catalogue alone, then another 38,792 /
158,892,032 for track 2 fallback or 76,659 / 313,995,264 for track 3. These are
4 KiB provider refills/returned bytes, not disk sectors or network traffic.
Preparation's maximum logical operation remains within 500,000 calls and 16 MiB;
aggregate cost spans explicitly bounded public calls and is separately reported.

## Storage and allocation

The API retains the existing reader and adds no index, metadata or media copy.
On the measured x86_64 Cursor<Vec<u8>>/SeekSource layout, SeekReader is 19,824
bytes, PreparedSource 19,840 bytes and a borrowed SeekSession 144 bytes. These
are target/generic-specific stack layouts, not stable ABI promises or retained
heap measurements. The existing 112-byte catalogue entries and 0.2.3 storage
tradeoff remain unchanged, including cost on chosen-cue-only sources and earlier
possible allocation failure under memory pressure.

All three allocation trials for each track/lifecycle report 80 requests and
25,304,113 requested bytes for both retained 0.2.3 and prepared 0.2.4. Requested
bytes include growth/transient allocations, not live heap. Whole-process peak
RSS KiB across the reference trials is below; advisory cases can include the
cache-advice helper's peak. The configured ceiling remains 131,072 KiB.

| Mode | Minimum RSS KiB | Maximum RSS KiB |
| --- | ---: | ---: |
| fresh | 11,824 | 12,084 |
| retained | 11,828 | 12,084 |
| prepared | 11,904 | 12,092 |

## Fresh-handle competitor refresh

A separate unchanged admitted matrix compares ordinary fresh SeekReader 0.2.3
and 0.2.4 with classic libwebm, libavformat, and Nestegg where admitted. There
are six queries (two mono, four six-hour silence), 26 jobs with seven warm and
seven advisory-evicted repetitions: 364 time/RSS trials plus 78 separate
allocation trials. The seed is 6110254; jobs use their prior common 64 KiB cache,
1 GiB address-space cap, 30 CPU/60 wall limits and pinned strict-window digest/
source-start-clamped encoded-preroll admission. This matrix uses the ordinary
reader, not prepared sessions. Hashes are pinned before and checked after runs.

Pins remain libwebm `a7c6662c68a90af18dd53dbb1f4262a346e9a547`, Nestegg
`767aab25013acefbdcc6d68a2a7a2c9081303e3f`, FFmpeg 7.0.2/libavformat 61.1.100.
[Earlier admission details](performance-0.2.1.md) remain applicable. No independent
prepared/reused-competitor lifecycle is admitted. Comparing a prepared unlace
session with a fresh competitor process would be unequal work. No general
performance ranking follows from these tables.

Fresh-handle first-packet milliseconds, median [minimum, maximum].


warm

| Source / target seconds | 0.2.3 | 0.2.4 | libwebm | libavformat | Nestegg |
| --- | ---: | ---: | ---: | ---: | ---: |
| mono / 0 | 0.062 [0.057, 0.084] | 0.061 [0.053, 0.086] | 0.041 [0.028, 0.054] | 0.074 [0.072, 0.102] | 0.027 [0.022, 0.035] |
| mono / 0.5 | 0.067 [0.063, 0.098] | 0.077 [0.063, 0.094] | 0.041 [0.035, 0.053] | 0.090 [0.078, 0.112] | 0.028 [0.024, 0.038] |
| six-hour / 0 | 7.267 [6.353, 14.360] | 6.668 [6.405, 11.093] | 4.075 [3.538, 7.422] | 0.102 [0.075, 0.127] | Not admitted |
| six-hour / 3600 | 7.172 [4.367, 8.156] | 6.218 [4.380, 7.925] | 4.855 [3.574, 7.269] | 40.783 [37.411, 75.503] | Not admitted |
| six-hour / 10800 | 4.624 [4.383, 8.626] | 7.376 [4.433, 9.440] | 3.758 [3.499, 7.168] | 112.270 [107.178, 221.087] | Not admitted |
| six-hour / 21599 | 7.586 [4.443, 9.789] | 6.643 [4.457, 8.572] | 6.845 [3.532, 8.838] | 423.081 [228.648, 574.559] | Not admitted |

advisory-evicted

| Source / target seconds | 0.2.3 | 0.2.4 | libwebm | libavformat | Nestegg |
| --- | ---: | ---: | ---: | ---: | ---: |
| mono / 0 | 0.316 [0.291, 0.333] | 0.308 [0.289, 0.324] | 0.284 [0.275, 0.300] | 0.321 [0.306, 0.378] | 0.288 [0.258, 0.420] |
| mono / 0.5 | 0.345 [0.308, 0.394] | 0.312 [0.306, 0.351] | 0.292 [0.268, 0.309] | 0.346 [0.327, 0.371] | 0.277 [0.251, 0.291] |
| six-hour / 0 | 14.109 [13.541, 21.077] | 13.934 [13.555, 18.539] | 10.236 [9.526, 12.044] | 0.462 [0.384, 1.159] | Not admitted |
| six-hour / 3600 | 11.794 [11.532, 14.785] | 11.830 [11.445, 41.861] | 9.703 [9.407, 12.015] | 39.890 [36.824, 78.697] | Not admitted |
| six-hour / 10800 | 12.045 [11.472, 13.748] | 12.042 [11.520, 18.013] | 11.087 [9.574, 14.791] | 121.879 [112.006, 227.152] | Not admitted |
| six-hour / 21599 | 12.266 [11.510, 14.188] | 12.162 [11.555, 15.849] | 9.774 [9.308, 11.946] | 231.706 [222.852, 233.990] | Not admitted |

Several competitors remain faster on admitted jobs. Small differences and broad
ranges are not evidence that the additive session API speeds the existing parser.
All measured unlace allocation request/byte counts in this ordinary-reader matrix
are unchanged between revisions. Unaffected sequential/writer speed data remain
historical reports, not relabeled as fresh 0.2.4 measurements.

## Readiness conclusion

The local prepared-source reference profile satisfies its measured per-opening
criterion on 112 previously unconsumed windows, including advisory-evicted data
and provider-cache resets. It requires a continuously owned original provider,
completed explicit preparation, fixed selection and exclusive sessions. It is a
candidate for a controlled consumer pilot that explicitly adopts those conditions;
there is no actual application rollout or owner-approved application SLA here.

New or independently reopened sources still require full validation, and their
first-use failures remain. This API does not supply disk-persistent indexes,
restart recovery, source authentication, a bounded network provider or decoder
restoration. The [readiness assessment](release-readiness.md) retains those scope
limits. The [experimental scope](experimental-scope.md) retains a monitored public
reporting contact and publication authorization as owner prerequisites. Raw media,
profiles, harnesses, pins and local records remain ignored under runs. No crate,
repository visibility, public tag or release artifact is published by this source
milestone.
