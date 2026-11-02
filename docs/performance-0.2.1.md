# Performance after reader buffer reuse in 0.2.1

Measured on 2026-11-02, unlace source `79a1ebe250dadd04cd9cabeacdee811574b6d2e2`, private Cargo
0.2.1. The preserved paired baseline is 0.2.0 production `0c4f9ce`. This report
keeps [0.2.0 results](performance-0.2.md) historical; it does not compare a fresh
number directly with an old run on a different schedule. The [profile and source
change](reader-performance.md) explain the refactoring and its retained 8 KiB
per-reader memory cost. Callgrind reductions are not substituted for real time.

## Work, qualification and pins

The unchanged pins are libwebm a7c6662c68a90af18dd53dbb1f4262a346e9a547,
Nestegg 767aab25013acefbdcc6d68a2a7a2c9081303e3f,
FFmpeg 7.0.2/libavformat 61.1.100, Symphonia 0.6.1 at
ee35874b571a35a9a6e15d3bc9a3aaf8f11fbeee and matroska-demuxer 0.8.1 at
e038e97b659ec2750b045a5f7a42e60b73c9491f. Native binaries and preserved 0.2.0
Rust executables match their prior hashes. Current research Cargo.lock differs
only in the local unlace version; third-party versions/sources are unchanged.
All source, executable, corpus and lockfile identities are checked after runs.

| Job | Qualified configurations | Contract |
| --- | ---: | --- |
| Strict selected audio | 46 | Original bytes/order, exact known presentation time, private bytes, raw delay/preroll and signed whole-block padding |
| Verified seek windows | 26 | Actual one-second selected-packet window, reported start and required encoded preroll, with separate opening/seek/read costs |
| Payload with std buffering | 12 | Complete selected bytes/order only; retains the earlier buffering configuration and its adverse overreads |
| Payload with common cache | 20 | Complete selected bytes/order with the same seek-aware 64 KiB cache; each implementation retains its own internal buffering |
| Writer counting sink | 4 | Existing packed/direct live/indexed serialization control, no file/codec work or cross-library writer ranking |

Strict admission retains the original qualified cases; earlier unsupported or
metadata-inexact counterpart cases remain excluded as documented previously.
New natural/large strict configurations all pass. Complete classic/Nestegg and
natural incremental manifests agree with current unlace. Additional complete
large incremental and both libavformat natural/large gates reach End. The latter
independently verify public sample-based delay/preroll/end discard and recover
raw ticks from libavformat's rounded declared-delay offset. These two sources
are entirely unlaced; this normalization is not generalized to arbitrary lacing,
negative raw clocks or fractional sample trims.

Every common-cache configuration passes a full streamed payload-hex comparison
and native End before timing, including both preserved/current unlace binaries.
This tier does not qualify counterpart timing, trimming or initialization. No
whole-library validation, decoding or completion guarantees are declared equal.
Natural data and the actual 2.92 GB mixed fixture, hashes/rights and the external
shortest-mux packet loss are described in [corpus qualification](natural-corpus.md).
The timed first audio track in the mixed fixture contains 969,806 packets;
standalone natural speech has 969,807. Six-hour silence has 1,080,001 packets
in only 10,399,259 bytes and is not diverse natural content.

Timed consumers use the same FNV-1a64 fields as the earlier protocol. Strict
initialization hashes private-length u64 LE, private bytes, delay/preroll u64 LE;
each packet hashes length u64 LE, bytes, known presentation i128 LE and whole-block
padding i64 LE. Payload jobs hash length and bytes only. Offset basis is
14695981039346656037 and prime 1099511628211 with wrapping u64 arithmetic. Every
timed run must match admitted packet count, payload count and digest; seek runs
also check their required start. Manifest serialization is not timed.

## Measurement conditions

There are **1,484 successful time/RSS trials and 324 separate allocation trials**.
Seven repetitions per time/cache configuration and three per allocation job use
fixed shuffled schedule 6110222. Warm inputs are explicitly read first; advisory
eviction uses POSIX_FADV_DONTNEED and is not proof of physical cold storage.
Only the counting-sink controls omit the second cache condition.

Host: AMD Ryzen AI 9 HX 370, Linux 7.0.0-38-generic x86_64, glibc 2.43. Each
measured child is pinned to logical CPU 0, limited to 1 GiB address space,
30 CPU seconds and 45 wall seconds, with core dumps disabled. Rust 1.89.0/LLVM
20.1.7 uses release opt-level 3 without LTO or target-cpu=native; native executables
retain the earlier GCC 15.2.0 build settings. No concurrent agent reviews,
compilation, tests, fuzzing or other task workloads run during measurement.
Desktop activity, turbo and clock frequency remain uncontrolled.

Elapsed time comes from the adapter's common work interval. GNU time records
whole-process peak RSS. A separate LD_PRELOAD probe counts allocation requests
and cumulative requested bytes; its elapsed time is not used. Counts are not live
heap size, and neither they nor process RSS erase the extra retained 8 KiB type
size. Minimum/maximum and medians are descriptive seven-run summaries, not
confidence intervals or a universal performance guarantee. Wide outliers remain.
The pinned schedule recovers allocation seek targets and checks every row identity.

## Paired current versus preserved unlace

Negative median change means lower elapsed time. All strict paired sources and
both cache conditions are retained, including small-input regressions.

| Source | Cache | 0.2.0 ms, median [min, max] | 0.2.1 ms, median [min, max] | Median change |
| --- | --- | ---: | ---: | ---: |
| browser-mixed | warm | 0.082 [0.072, 0.088] | 0.086 [0.073, 0.116] | +5.9% |
| browser-mixed | advisory-evicted | 0.298 [0.230, 0.345] | 0.298 [0.236, 0.363] | -0.0% |
| fixed-padding-25000000 | warm | 0.038 [0.032, 0.041] | 0.039 [0.032, 0.078] | +3.5% |
| fixed-padding-25000000 | advisory-evicted | 0.251 [0.178, 0.490] | 0.187 [0.176, 0.350] | -25.5% |
| large-mixed | warm | 663.932 [590.300, 1252.917] | 634.000 [533.598, 1048.844] | -4.5% |
| large-mixed | advisory-evicted | 2239.409 [2180.276, 2526.793] | 2201.968 [2173.558, 2540.896] | -1.7% |
| mono | warm | 0.064 [0.062, 0.094] | 0.061 [0.056, 0.086] | -4.0% |
| mono | advisory-evicted | 0.265 [0.213, 0.448] | 0.279 [0.210, 0.393] | +5.3% |
| natural-speech | warm | 307.882 [296.404, 378.723] | 249.358 [221.610, 542.886] | -19.0% |
| natural-speech | advisory-evicted | 305.508 [301.181, 374.665] | 241.747 [231.972, 270.698] | -20.9% |
| negative-raw-time | warm | 0.041 [0.033, 0.066] | 0.037 [0.033, 0.049] | -10.4% |
| negative-raw-time | advisory-evicted | 0.218 [0.167, 0.256] | 0.188 [0.184, 0.296] | -13.9% |
| none-padding-10000000 | warm | 0.040 [0.035, 0.055] | 0.039 [0.036, 0.259] | -1.1% |
| none-padding-10000000 | advisory-evicted | 0.244 [0.161, 0.339] | 0.264 [0.167, 1.087] | +8.3% |
| six-hour | warm | 284.083 [255.103, 653.911] | 199.221 [175.353, 317.055] | -29.9% |
| six-hour | advisory-evicted | 253.969 [247.736, 280.761] | 185.092 [178.642, 205.349] | -27.1% |

In warm strict six-hour processing, the medians are 284.083 to 199.221 ms,
29.9% lower. Natural speech is 307.882 to 249.358 ms, 19.0% lower. The large mixed
median is only 4.5% lower and has heavily overlapping ranges; that is not strong
evidence of a stable large-file speedup. Small advisory-evicted jobs can regress.
The first packet need not improve: for example strict mono advisory-evicted
median first latency is 242.9 to 260.6 us, and six-hour is 353.3 to 416.4 us.
These tradeoffs are not hidden by the long-loop throughput result.

Common-cache payload pairs use a different consumer and must stay separate:

| Source | Cache | 0.2.0 ms, median [min, max] | 0.2.1 ms, median [min, max] | Median change |
| --- | --- | ---: | ---: | ---: |
| large-mixed | warm | 613.636 [592.869, 800.773] | 525.744 [496.075, 1064.691] | -14.3% |
| large-mixed | advisory-evicted | 2228.067 [2185.317, 2625.183] | 2177.921 [2162.097, 2578.038] | -2.3% |
| mono | warm | 0.069 [0.066, 0.100] | 0.064 [0.055, 0.086] | -7.6% |
| mono | advisory-evicted | 0.237 [0.224, 0.353] | 0.289 [0.208, 0.319] | +21.9% |
| natural-speech | warm | 288.094 [274.562, 313.545] | 237.698 [221.863, 402.723] | -17.5% |
| natural-speech | advisory-evicted | 296.250 [289.320, 342.529] | 227.109 [223.188, 272.910] | -23.3% |
| six-hour | warm | 257.861 [241.121, 392.399] | 187.377 [178.599, 399.362] | -27.3% |
| six-hour | advisory-evicted | 245.419 [237.190, 259.590] | 164.885 [162.672, 185.493] | -32.8% |
| synthetic-mixed | warm | 0.057 [0.052, 0.083] | 0.054 [0.048, 0.076] | -5.1% |
| synthetic-mixed | advisory-evicted | 0.265 [0.207, 0.308] | 0.272 [0.197, 0.308] | +2.6% |

## Pinned counterpart comparisons

The following warm strict rows retain time/latency ranges, RSS ranges, separate
allocation counts and actual backing-read requests. They are selected-audio
consumer comparisons, not equal parser feature/validation sets.

| Source | Reader | ms, median [min, max] | First packet us [min, max] | Peak RSS KiB [min, max] | Allocation requests | Backing MB |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| six-hour | nestegg | 67.283 [61.900, 73.756] | 22.8 [17.4, 32.8] | 1948 [1756, 2080] | 4,320,029 | 10.40 |
| six-hour | libwebm-incremental | 101.828 [99.466, 246.344] | 60.7 [57.0, 84.3] | 4536 [4512, 4676] | 1,253,205 | 10.40 |
| six-hour | libwebm | 159.828 [149.441, 437.025] | 3447.2 [3127.9, 15853.0] | 236100 [235892, 236108] | 2,203,226 | 27.26 |
| six-hour | unlace 0.2.1 | 199.221 [175.353, 317.055] | 43.2 [38.0, 54.3] | 2548 [2344, 2580] | 23 | 10.40 |
| six-hour | libavformat | 252.184 [232.915, 450.032] | 79.9 [74.1, 96.9] | 4232 [4164, 4296] | 7,560,169 | 10.03 |
| natural-speech | nestegg | 130.636 [126.396, 184.930] | 24.2 [22.0, 41.9] | 1952 [1948, 2080] | 3,879,288 | 83.36 |
| natural-speech | libwebm-incremental | 191.949 [141.836, 392.653] | 80.1 [63.5, 93.1] | 4528 [4524, 4636] | 1,009,033 | 83.03 |
| natural-speech | libwebm | 206.439 [191.429, 271.753] | 2988.6 [2549.4, 6293.4] | 148932 [148696, 148940] | 1,947,414 | 151.13 |
| natural-speech | unlace 0.2.1 | 249.358 [221.610, 542.886] | 42.0 [30.4, 65.0] | 2540 [2516, 2644] | 25 | 83.03 |
| natural-speech | libavformat | 294.278 [271.260, 565.982] | 75.8 [54.9, 102.9] | 4232 [4164, 4360] | 6,788,855 | 82.97 |
| large-mixed | nestegg | 332.418 [309.200, 382.199] | 39.4 [35.4, 54.7] | 1952 [1948, 2080] | 7,913,703 | 3032.02 |
| large-mixed | libwebm-incremental | 354.394 [344.473, 412.322] | 76.6 [71.1, 81.4] | 4528 [4500, 4696] | 2,289,172 | 2543.10 |
| large-mixed | libwebm | 575.177 [539.270, 632.265] | 100205.9 [90026.1, 139519.1] | 425764 [425628, 425776] | 4,034,452 | 5084.68 |
| large-mixed | libavformat | 624.263 [602.371, 665.688] | 101.0 [95.9, 104.9] | 6344 [6276, 6472] | 13,849,319 | 2920.14 |
| large-mixed | unlace 0.2.1 | 634.000 [533.598, 1048.844] | 61.3 [58.6, 93.7] | 2540 [2524, 2572] | 31 | 2920.93 |

Unlace remains slower than Nestegg and both libwebm paths on these long strict
cases. Its six-hour/natural medians are lower than libavformat's in this schedule;
the large mixed medians are close with broad overlap. No general superiority
claim follows. Classic libwebm retains a full directory and has much higher RSS;
Nestegg's whole-process RSS is lower than unlace's here despite many cumulative
allocation requests. Low allocation count alone is neither lowest memory nor
fastest iteration.

Payload-only Rust readers under the common cache:

| Source | Reader | ms, median [min, max] | First packet us [min, max] | Peak RSS KiB [min, max] | Allocation requests | Backing MB |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| mono | unlace 0.2.1 | 0.064 [0.055, 0.086] | 41.8 [36.1, 59.7] | 3244 [3204, 3348] | 25 | 0.01 |
| mono | matroska | 0.071 [0.064, 0.094] | 54.6 [48.5, 71.7] | 3380 [3320, 3468] | 72 | 0.01 |
| mono | symphonia | 0.122 [0.106, 0.170] | 104.3 [89.6, 146.6] | 3540 [3400, 3596] | 196 | 0.01 |
| synthetic-mixed | unlace 0.2.1 | 0.054 [0.048, 0.076] | 43.8 [38.3, 60.0] | 3268 [3156, 3400] | 26 | 0.01 |
| synthetic-mixed | matroska | 0.070 [0.062, 0.075] | 61.3 [53.1, 65.2] | 3344 [3248, 3500] | 91 | 0.01 |
| synthetic-mixed | symphonia | 0.112 [0.105, 0.146] | 99.9 [92.7, 130.8] | 3496 [3404, 3648] | 171 | 0.01 |
| six-hour | symphonia | 90.519 [82.498, 152.864] | 87.4 [76.7, 100.5] | 3452 [3416, 3612] | 2,160,046 | 10.09 |
| six-hour | matroska | 131.742 [123.685, 139.916] | 9852.2 [9187.0, 10474.9] | 5448 [5384, 5548] | 43,265 | 20.93 |
| six-hour | unlace 0.2.1 | 187.377 [178.599, 399.362] | 46.0 [39.2, 52.4] | 3228 [3096, 3416] | 24 | 10.40 |
| natural-speech | symphonia | 159.140 [142.772, 248.931] | 825.6 [736.2, 1435.6] | 3904 [3772, 4012] | 1,939,725 | 83.18 |
| natural-speech | matroska | 172.859 [159.611, 194.355] | 1529.3 [1476.0, 1955.4] | 3752 [3664, 3880] | 7,854 | 83.20 |
| natural-speech | unlace 0.2.1 | 237.698 [221.863, 402.723] | 44.0 [41.0, 59.5] | 3316 [3172, 3324] | 26 | 83.03 |
| large-mixed | matroska | 430.473 [419.076, 439.007] | 14484.2 [14019.8, 15690.6] | 7224 [7128, 7356] | 77,704 | 2921.86 |
| large-mixed | symphonia | 465.096 [443.307, 700.021] | 7055.7 [6448.7, 16998.9] | 5128 [4944, 5260] | 3,956,947 | 2921.10 |
| large-mixed | unlace 0.2.1 | 525.744 [496.075, 1064.691] | 63.2 [58.7, 92.4] | 3180 [3144, 3404] | 32 | 2920.93 |

Symphonia and matroska-demuxer still beat current unlace medians on all three long
payload jobs shown. Some ranges overlap or contain substantial outliers. Their
public metadata APIs remain outside the strict tier; byte equality is not an
unreported timing/trim equivalence claim.

The std-buffering six-hour medians are matroska-demuxer 2959.305 ms,
Symphonia 93.250 ms and current unlace 177.068 ms. Matroska-demuxer's common-cache
median is 131.742 ms. Its large apparent std-buffering loss is substantially an
adapter interaction, as in the earlier report; it is not an intrinsic unlace
parser victory. Backing bytes are requested bytes at the underlying adapter,
not physical storage traffic.

## Seek and serialization controls

Opening, seek and subsequent window reads are recorded separately. The paired
unlace seek entries below retain both conditions; the full seek matrix
keeps 26 admitted configurations with their required starts, byte/call counts and
all ranges in the local accepted summary.

| Source | Request seconds | Cache | Open us old/new | Seek us old/new | Total ms old/new |
| --- | ---: | --- | ---: | ---: | ---: |
| mono | 0 | advisory-evicted | 231.6 / 260.4 | 2.4 / 2.8 | 0.261 / 0.287 |
| mono | 0 | warm | 50.0 / 48.8 | 2.9 / 2.7 | 0.082 / 0.077 |
| mono | 0.5 | advisory-evicted | 224.5 / 227.6 | 6.2 / 6.2 | 0.254 / 0.254 |
| mono | 0.5 | warm | 53.7 / 61.1 | 6.6 / 9.0 | 0.083 / 0.095 |
| six-hour | 0 | advisory-evicted | 11559.6 / 11274.6 | 2562.3 / 2499.4 | 14.148 / 13.775 |
| six-hour | 0 | warm | 4197.1 / 4539.7 | 2895.5 / 2740.1 | 7.119 / 7.441 |
| six-hour | 10800 | advisory-evicted | 11455.2 / 11139.6 | 71.7 / 67.1 | 11.557 / 11.268 |
| six-hour | 10800 | warm | 4053.1 / 4270.4 | 75.5 / 70.8 | 4.165 / 4.377 |
| six-hour | 21599 | advisory-evicted | 11885.4 / 12096.7 | 80.3 / 75.0 | 12.036 / 12.237 |
| six-hour | 21599 | warm | 4234.9 / 4271.9 | 107.1 / 76.9 | 4.408 / 4.446 |
| six-hour | 3600 | advisory-evicted | 12146.6 / 12223.5 | 65.3 / 64.0 | 12.245 / 12.313 |
| six-hour | 3600 | warm | 3938.0 / 4329.8 | 66.6 / 72.4 | 4.038 / 4.430 |

These short windows are dominated by opening/provider/index work rather than
long packet iteration; not every median improves. No decoder restoration or
network/provider latency claim is implied.

| Counting sink | ms median [min, max] | Blocks / Clusters | Sink writes | Allocation requests |
| --- | ---: | ---: | ---: | ---: |
| indexed-direct | 79.405 [76.956, 102.223] | 1,080,001 / 1,080,001 | 3,261,608 | 38 |
| indexed-packed | 74.087 [69.123, 122.260] | 1,080,001 / 21,601 | 64,807 | 44 |
| live-direct | 74.792 [71.141, 181.195] | 1,080,001 / 1,080,001 | 3,240,004 | 24 |
| live-packed | 66.852 [64.074, 173.982] | 1,080,001 / 21,601 | 43,203 | 30 |

The reader refactor does not alter writer layout; all fourteen fixed remux outputs
retain their earlier hashes. These synthetic controls are not file-write latency,
atomic publication, storage durability or a competing muxer's benchmark.

## Reproduction and readiness limits

Use the pinned source/compiler/counterpart revisions and corpus identities;
build current adapters separately without replacing preserved binaries. Reproduce
the documented exact fields and full payload/End gates first, then the seeded
single-child schedule, seven time/RSS repetitions for both cache conditions and
three separate allocation repetitions. Verify all artifact/input/source pins,
required seek starts and counts/digests after the run. Raw scripts, source copies,
per-trial rows, profiles and media stay local under repository policy; only these
sanitized methods and aggregate conclusions are tracked.

This is one host and a limited set of accepted inputs. It does not define a
supported platform matrix or owner-approved application SLA. The refactor reduces
measured long-loop costs while retaining exact container guarantees and an
explicit memory tradeoff. Consumer integration, input/platform acceptance and
failure/durability policies still require a bounded pilot; publication/license
choices remain owner decisions in the [readiness assessment](release-readiness.md).
