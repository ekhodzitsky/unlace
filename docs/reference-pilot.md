# Bounded library reference pilot

Assessed on 2026-11-02 for private unlace 0.2.1, production source `79a1ebe`.
The owner has no selected application workload or acceptance budget. These are
explicit provisional library-consumer profiles on the recorded Linux x86_64
host; no other project, real network service or production deployment is changed.
They support a concrete integration discussion, not general production approval.

## Profiles and fixed budgets

| Profile | Source, selection and consumer |
| --- | --- |
| Sequential | Immutable local WebM/Opus mono/stereo; small files, six-hour silence, 5.39-hour natural speech and both audio tracks of the 2.92 GB mixed fixture; read through observed End |
| Paused | Captured audio/mixed Chromium MediaRecorder; 17-byte reads alternate with WouldBlock, one reader resumes after explicit approximately 1 ms producer waits; unknown Segment completion stays explicit |
| Ranged | Owned immutable local FileExt provider, revision 1, 4 KiB cache for sparse header access; first/middle one-second audio windows, with a separate injected NeedRange during prepare, seek and packet polling |
| Staged writing | Existing remux example, finite natural/large inputs, selected audio, explicit finish/file sync and same-directory no-replace publication; live versus indexed output remains distinct |

Fixed before runs: 128 MiB process address space and observed RSS ceiling,
30 CPU seconds, 60 seconds supervisor wall time, sources at most 3 GiB and
fixture membership up to six hours of intended audio. Duration is not a trusted
container-field limit or a changed library default. The first consumed packet
has a provisional 250 ms active-wall budget, subtracting only measured explicit
producer/range sleeps. This is not CPU time and still includes I/O and scheduling.

Demux byte/work limits stay at defaults. Ranged cases choose either the default
65,536-call or explicit 500,000-call read allowance per operation, with the default
16 MiB logical bytes per operation and 1,000,000 scan headers per seek. No limit refusal is automatically retried with
a larger value. Range resumes are capped at 16, preparation calls at 10,000 and
window-observed packets at 100,000. Each positive ranged case exercises exactly
three controlled holes/resumes without changing the revision token. The reference
cache uses 4 KiB for sparse prefix access; it is separate from the performance
report's common 64 KiB comparison cache.

The consumers retain a rolling digest/count rather than a packet list. The digest
covers private bytes, delay/preroll and each selected packet's length/bytes,
known presentation time and whole-block padding. It does not hash every Packet
field: raw clocks, flags, lace fields, durations, once-only padding and UID remain
covered by the separate [full qualification](reader-performance.md), not silently
added to this pilot's oracle. The preserved qualified 0.2.0 library supplies
reference consumer results; its encoded/native/PCM admission remains separate.

Ranged consumers normally stop after their selected window and report no suffix
End. They check the actual/reported start, fallback, required preroll and window
fields; they do not validate a complete suffix or decoder restoration. Eight
complete suffix comparisons exist in the separate corpus/refactor evidence.

## Results, including failed criteria

The initial 42-trial cohort has 39 successful cases and three explicit call-budget
refusals of the large file's second audio track. Its wrappers did not guarantee
that killing GNU time would also kill its child. Review reproduced that tooling
limitation. The corrected supervisor owns a POSIX session/process group and kills
that group on timeout. A controlled before/after test shows the old wrapper kill
leaves a sleeping descendant; the new control returns timeout status 124 in
0.304 seconds with no running descendant. This covers ordinary owned descendants,
not hostile session escape or an OS hard real-time guarantee.

The second, supervised 42-trial cohort has **38 successful cases, three call-budget
refusals and one latency-budget failure**. All evaluated packet-field digests
match. The failed criteria are retained, and no budget was raised to make them
pass. Three repetitions per configuration are summarized below.

| Profile | Input/track | Pass / 3 | Call refusals | Other failures | First active ms min–max | Max RSS KiB | Max supervisor seconds |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| paused | browser-audio | 3 | 0 | 0 | 0.156–0.338 | 2,600 | 1.721 |
| paused | browser-mixed | 3 | 0 | 0 | 0.319–0.494 | 2,640 | 2.105 |
| ranged | large-audio2 | 2 | 0 | 1 | 63.024–521.320 | 9,612 | 0.566 |
| ranged | large-audio3 | 0 | 3 | 0 | not reached | 9,612 | 0.241 |
| ranged | mono | 3 | 0 | 0 | 0.073–0.107 | 2,700 | 0.014 |
| ranged | natural-mono | 3 | 0 | 0 | 3.286–12.394 | 3,468 | 0.039 |
| ranged | six-hour | 3 | 0 | 0 | 9.777–33.604 | 6,404 | 0.054 |
| ranged | stereo | 3 | 0 | 0 | 0.072–0.140 | 2,696 | 0.009 |
| sequential | large-audio2 | 3 | 0 | 0 | 0.102–0.119 | 2,648 | 1.335 |
| sequential | large-audio3 | 3 | 0 | 0 | 0.088–0.107 | 2,640 | 2.427 |
| sequential | mono | 3 | 0 | 0 | 0.043–0.051 | 2,648 | 0.008 |
| sequential | natural-mono | 3 | 0 | 0 | 0.068–0.070 | 2,584 | 0.838 |
| sequential | six-hour | 3 | 0 | 0 | 0.067–0.079 | 2,584 | 0.839 |
| sequential | stereo | 3 | 0 | 0 | 0.051–0.053 | 2,552 | 0.010 |

The large mixed input has video-only Cues. Seeking its second audio track at the
chosen 500,000-call budget returns Source(Limit) in both preserved/current
implementations. A same-budget diagnostic confirms exactly 500,000 calls and
773,298 logical bytes at refusal, below the 16 MiB byte allowance. This is a failed application-profile case, not packet corruption
or a new regression. The first audio track also refuses a separate before-first
request at the public default 65,536-call budget. Both refusals stay explicit.

For the first audio track at the larger budget, one supervised trial reaches the
correct window in 521.320 ms active wall time, exceeding the fixed 250 ms criterion;
the other two are lower. Initial cohort maxima were lower, but the cohorts do not
establish a stable latency guarantee. Cache state/background scheduling were not
controlled per trial: input hashing/baseline passes affect cache residency, and
the later cohort reuses baseline fields without rerunning that warmup. The recorded slow case requests 318,910,776 backing bytes in 77,860 cache
refills while preparing/seeking/reading its small window. These are provider
requests, not physical disk traffic; logical read allowances do not cap extra
provider prefetch costs. No causal claim about the outlier is made. Staging work had already completed before this
miss, so it is not attributed to that work.

All observed reading trials stay below the RSS and 60-second wall ceilings.
These fixed-file results do not validate HTTP request latency, cache eviction,
provider deadlines, authentication, source replacement, arbitrary codecs or a
platform matrix. Snapshot honesty and bounded provider methods remain consumer
contracts. The core's explicit refusals are preserved.

## Staged output policy

Three completed outputs match the selected input digest/counts, with expected
finite indexed versus unknown-size live completion on readback:

| Output | Bytes | Packets | Wall seconds | RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| natural-indexed | 84,242,460 | 969,807 | 0.355 | 3,136 |
| natural-live | 83,117,299 | 969,807 | 0.390 | 2,568 |
| large-stereo-indexed | 122,333,177 | 969,807 | 0.690 | 3,068 |

The writing jobs use a 1 GiB output-file allowance and the same memory/CPU/wall
ceilings. Successful receipts and file sync precede hard-link publication. Normal
staging names are removed afterward. Attempting an existing destination fails
without replacing its hash; the retained completed staging file is hashed and
then deliberately cleaned inside the local pilot directory. A late truncated
input leaves staging and never publishes its destination. Injected sink failures
remain covered by the ordinary/fuzz writer qualification rather than a claimed
real full-filesystem or power-loss trial.

File sync and no-replace visibility do not establish directory crash durability.
The portable example explicitly leaves that responsibility to a consumer/storage
policy. No such durability approval or rollback of already visible generic sink
bytes is inferred from these successful tests.

## Provenance and disposition

Consumer source, consumer executable and input identities were recorded before
the original runs; unchanged inputs/executables are checked afterward. Initial
continuation-runner and publication wrapper/executable hashes were not all
pre-recorded. Their expanded hash inventory is explicitly retrospective, as are
the later wrapper hash records; it is not described as pre-execution attestation.
Raw cohorts, failures, hashes, sources, logs and media remain local. Only accepted
sanitized conclusions are tracked. The [second-agent review](independent-review.md)
checked scope, failures, timeout control and publication evidence.

The pilot is a completed evaluation with failed criteria, not a production
rollout or blanket pass. The owner still needs an actual input/provider/platform
profile and acceptance budgets. The observed large ranged cases fail the chosen
reference envelope; selecting a different explicit policy requires evaluating its
costs and guarantees. The owner subsequently selected MIT; publication remains
a separate decision in the
[readiness assessment](release-readiness.md).
