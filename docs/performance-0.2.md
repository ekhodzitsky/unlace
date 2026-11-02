# Performance after the 0.2.0 source audit

Measured on 2026-11-02 against unlace `0c4f9ce0db9ddd1cc55a25fa162be1c4050bb958` (private Cargo 0.2.0).
This refresh keeps the [historical report](matched-comparison.md) unchanged.
It compares explicitly different consumer jobs below, not whole-library quality,
identical validation/completeness policies, codec decoding or production readiness.

## Pins, equivalence and measurement

The pinned revisions are unchanged: libwebm
`a7c6662c68a90af18dd53dbb1f4262a346e9a547`, Nestegg
`767aab25013acefbdcc6d68a2a7a2c9081303e3f`, FFmpeg 7.0.2/libavformat 61.1.100,
Symphonia 0.6.1 at `ee35874b571a35a9a6e15d3bc9a3aaf8f11fbeee`, and
matroska-demuxer 0.8.1 at `e038e97b659ec2750b045a5f7a42e60b73c9491f`.
See the historical report for source/license/archive/build details and full corpus
hashes/rights. Native C/C++ executable SHA-256 values are verified unchanged before
and after these trials. Current Rust adapters are built separately, preserving
all historical binaries and raw measurements.

Hardware: AMD Ryzen AI 9 HX 370 w/ Radeon 890M; logical CPU 0 affinity for each measured child.
OS: Linux-7.0.0-38-generic-x86_64-with-glibc2.43, glibc 2.43. Rust 1.89.0/LLVM 20.1.7,
release optimization level 3, no LTO or target-cpu=native. Native binaries retain
GCC 15.2.0 C11/C++11 -O3 and the historical static FFmpeg configuration.
Background activity, turbo and clock frequency are uncontrolled; this is one
machine. Seven repetitions per time/RSS condition, three separate allocation
repetitions, fixed randomized schedule seed 6110203, one measured child at a time.
No concurrent tests/compilation/fuzzing are performed during measurement.

Each child is capped at 1 GiB address space, 30 CPU seconds and 45 wall seconds;
core dumps are disabled. Warm input is explicitly read first; advisory eviction
uses POSIX_FADV_DONTNEED per file. It is not a physical-device cold-cache proof.
RSS is whole-process GNU time peak RSS. Allocation requests/bytes use the same
separate LD_PRELOAD probe as the historical report; instrumented elapsed time is
not used for timing. Requests are not live allocations or peak heap bytes.

A second fixed schedule, seed 6110204, qualifies and measures the common-cache
Rust configuration with the same repetitions and bounds.

The final records contain 952 successful time/RSS trials and 210 allocation
trials. A post-run audit checks every expected count/digest/preroll, repetition
set, source/executable pin and unchanged source commit. Allocation rows retain
execution order; the pinned deterministic schedule attaches each seek target in
the derived record, with row identity checked at every position. No raw timing
or allocation row is replaced by a derived summary.

There are three admission tiers plus a separate serialization experiment:

- Strict selected-audio consumer: all original bytes/order/track, exact known
  timestamps, CodecPrivate, raw delay/preroll and signed whole-block padding match
  complete reference manifests before timing. Every timed run checks the common
  FNV64 field consumer, packet count and payload byte count. Hashing is timed;
  manifest serialization is not. This yields 28 jobs. Rust counterpart public
  metadata still fails this tier, not a speed test or an unavailable build.
- Verified seek windows: 20 jobs reproduce a one-second selected-packet window and
  required encoded preroll before timing. Every run checks its count/digest and
  actual first timestamp. Opening/indexing, seek and subsequent reads are separate.
  This does not establish decoder readiness or repeated-seek state correctness.
- Payload-only Rust consumer: nine baseline plus nine common-cache jobs compare all packet bytes/order and selected
  audio on mono, mixed VP9/Opus and six-hour finite sources with unlace, Symphonia
  and matroska-demuxer. These sources each have one enabled audio track, with known
  mapping through mixed video. A fresh complete streamed hex comparison precedes
  timing and native End is required. Timed work hashes ordered length plus payload
  only. It deliberately makes no exact time/trim/init equivalence claim.
- Writer counting sink: four jobs submit 1,080,001 identical already encoded Opus
  packets at 20 ms spacing, live/indexed and packed/direct. Receipt counts, offsets,
  Cluster/cue counts and sink calls are checked. The sink retains no bytes and
  performs no filesystem or codec work. This is a writer layout experiment, not
  a cross-library muxer comparison, decoded six-hour duration proof or storage
  throughput result. Independent writer output qualification remains separate.

The strict adapters are unchanged consumer source linked to audited unlace; their
timers stop before parser destruction. The payload-only Rust adapter includes
parser destruction in its total time for all three backends, while recording
opening and first-packet times separately. Input buffering is 64 KiB for unlace
and matroska-demuxer; Symphonia uses its 64 KiB MediaSourceStream ring and native
growing read-ahead. Runtime/provider behavior remains library-specific. Do not
compare absolute totals across tiers as though they were the same consumer job.

## Strict selected-audio timing

Milliseconds are median [minimum, maximum] of seven runs. First-packet times and
RSS below are warm medians. Throughput is full input MiB per median elapsed second.
The six-hour source is only 10,399,259 bytes with 1,080,001 repeated silence packets;
it is not six hours of diverse speech or a multi-gigabyte input.

| Source | Adapter | Warm total ms | Advisory-evicted total ms | Warm first ms | Warm MiB/s |
| --- | --- | --- | --- | ---: | ---: |
| browser-mixed | FFmpeg libavformat | 0.106 [0.087, 0.129] | 0.266 [0.251, 0.284] | 0.073 | 201.30 |
| browser-mixed | libwebm classic | 0.063 [0.058, 0.065] | 0.227 [0.214, 0.301] | 0.046 | 338.14 |
| browser-mixed | libwebm incremental | 0.091 [0.076, 0.122] | 0.253 [0.229, 0.280] | 0.061 | 232.91 |
| browser-mixed | nestegg | 0.047 [0.040, 0.060] | 0.206 [0.195, 0.210] | 0.025 | 452.92 |
| browser-mixed | unlace | 0.071 [0.065, 0.561] | 0.246 [0.227, 0.263] | 0.038 | 298.32 |
| fixed-padding-25000000 | FFmpeg libavformat | 0.062 [0.055, 0.089] | 0.210 [0.199, 0.369] | 0.061 | 3.47 |
| fixed-padding-25000000 | libwebm classic | 0.027 [0.026, 0.042] | 0.171 [0.166, 0.194] | 0.027 | 7.88 |
| fixed-padding-25000000 | libwebm incremental | 0.059 [0.049, 0.080] | 0.245 [0.198, 0.364] | 0.059 | 3.65 |
| fixed-padding-25000000 | nestegg | 0.018 [0.013, 0.025] | 0.163 [0.158, 0.230] | 0.018 | 11.92 |
| fixed-padding-25000000 | unlace | 0.037 [0.031, 0.050] | 0.181 [0.178, 0.422] | 0.036 | 5.92 |
| mono | FFmpeg libavformat | 0.086 [0.067, 0.090] | 0.262 [0.240, 0.838] | 0.064 | 121.95 |
| mono | libwebm classic | 0.047 [0.044, 0.062] | 0.209 [0.205, 0.248] | 0.031 | 224.83 |
| mono | libwebm incremental | 0.081 [0.076, 0.104] | 0.243 [0.234, 0.282] | 0.062 | 130.02 |
| mono | nestegg | 0.030 [0.028, 0.034] | 0.199 [0.184, 0.285] | 0.018 | 350.83 |
| mono | unlace | 0.064 [0.058, 0.077] | 0.227 [0.205, 1.936] | 0.042 | 164.12 |
| negative-raw-time | libwebm classic | 0.026 [0.024, 0.039] | 0.178 [0.166, 0.271] | 0.026 | 7.50 |
| negative-raw-time | libwebm incremental | 0.059 [0.038, 0.073] | 0.209 [0.194, 3.627] | 0.059 | 3.28 |
| negative-raw-time | unlace | 0.036 [0.028, 0.055] | 0.182 [0.179, 2.964] | 0.035 | 5.45 |
| none-padding-10000000 | FFmpeg libavformat | 0.061 [0.037, 0.087] | 0.201 [0.189, 3.497] | 0.059 | 3.46 |
| none-padding-10000000 | libwebm classic | 0.030 [0.027, 0.043] | 0.171 [0.165, 0.266] | 0.029 | 7.05 |
| none-padding-10000000 | libwebm incremental | 0.054 [0.038, 0.059] | 0.212 [0.197, 0.276] | 0.054 | 3.89 |
| none-padding-10000000 | nestegg | 0.016 [0.015, 0.026] | 0.168 [0.158, 0.232] | 0.016 | 12.76 |
| none-padding-10000000 | unlace | 0.038 [0.035, 0.051] | 0.238 [0.176, 0.252] | 0.037 | 5.59 |
| six-hour | FFmpeg libavformat | 230.181 [227.082, 383.593] | 233.372 [228.740, 237.413] | 0.067 | 43.09 |
| six-hour | libwebm classic | 149.925 [148.415, 154.760] | 157.665 [156.331, 170.379] | 3.188 | 66.15 |
| six-hour | libwebm incremental | 96.872 [95.770, 99.280] | 98.967 [97.813, 102.483] | 0.056 | 102.38 |
| six-hour | nestegg | 62.929 [61.333, 64.089] | 64.928 [63.329, 88.357] | 0.020 | 157.60 |
| six-hour | unlace | 252.705 [243.422, 273.524] | 257.913 [253.332, 433.846] | 0.037 | 39.25 |

Warm resource medians use the same seven uninstrumented runs; allocation medians
use three separate instrumented processes. Read bytes are observed backing reads,
not network traffic, physical disk reads or proof of verified media coverage.

| Source | Adapter | Peak RSS KiB | Allocation requests | Requested bytes | Read bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| browser-mixed | FFmpeg libavformat | 3200 | 488 | 132975 | 22330 |
| browser-mixed | libwebm classic | 4316 | 131 | 248866 | 22330 |
| browser-mixed | libwebm incremental | 4484 | 539 | 181425 | 22330 |
| browser-mixed | nestegg | 2072 | 248 | 43272 | 22330 |
| browser-mixed | unlace | 2564 | 23 | 70835 | 22330 |
| fixed-padding-25000000 | FFmpeg libavformat | 3136 | 62 | 78903 | 227 |
| fixed-padding-25000000 | libwebm classic | 4316 | 21 | 168825 | 227 |
| fixed-padding-25000000 | libwebm incremental | 4496 | 406 | 173014 | 227 |
| fixed-padding-25000000 | nestegg | 1752 | 33 | 14472 | 227 |
| fixed-padding-25000000 | unlace | 2552 | 22 | 70037 | 227 |
| mono | FFmpeg libavformat | 3072 | 457 | 120266 | 11009 |
| mono | libwebm classic | 4316 | 141 | 174897 | 11009 |
| mono | libwebm incremental | 4488 | 468 | 178919 | 11009 |
| mono | nestegg | 1944 | 268 | 31752 | 11009 |
| mono | unlace | 2568 | 24 | 70869 | 11009 |
| negative-raw-time | libwebm classic | 4316 | 21 | 168769 | 204 |
| negative-raw-time | libwebm incremental | 4488 | 404 | 172836 | 204 |
| negative-raw-time | unlace | 2544 | 22 | 70026 | 204 |
| none-padding-10000000 | FFmpeg libavformat | 3136 | 50 | 78156 | 220 |
| none-padding-10000000 | libwebm classic | 4316 | 21 | 168793 | 220 |
| none-padding-10000000 | libwebm incremental | 4488 | 404 | 172980 | 220 |
| none-padding-10000000 | nestegg | 1880 | 29 | 14402 | 220 |
| none-padding-10000000 | unlace | 2520 | 22 | 70030 | 220 |
| six-hour | FFmpeg libavformat | 4288 | 7560169 | 466021482 | 10027030 |
| six-hour | libwebm classic | 236060 | 2203226 | 282842805 | 27262997 |
| six-hour | libwebm incremental | 4496 | 1253205 | 98496271 | 10399259 |
| six-hour | nestegg | 1940 | 4320029 | 141494402 | 10399259 |
| six-hour | unlace | 2516 | 23 | 70099 | 10399259 |

## Payload-only Rust timing

This is a narrower, explicitly qualified packet-extraction job. All three readers
produce identical complete selected payload streams on these three finite inputs.
The baseline uses standard input buffering, not the common cache below.
The earlier exact metadata limitations remain; speed here does not make their
raw timing/trim APIs interchangeable. Table intervals are seven runs, with warm
first/RSS and three-run allocation medians.

| Source | Adapter | Warm total ms | Advisory-evicted total ms | First ms | RSS KiB | Allocation requests | Read bytes |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| mono | matroska-demuxer | 0.135 [0.130, 0.249] | 0.309 [0.291, 0.336] | 0.079 | 3292 | 72 | 1097367 |
| mono | symphonia | 0.107 [0.095, 0.196] | 0.270 [0.247, 3.723] | 0.088 | 3424 | 195 | 11559 |
| mono | unlace | 0.061 [0.059, 0.077] | 0.226 [0.219, 0.331] | 0.040 | 3196 | 25 | 11009 |
| six-hour | matroska-demuxer | 2669.138 [2630.207, 2835.529] | 2652.266 [2621.190, 3074.193] | 109.897 | 5412 | 43265 | 148330957082 |
| six-hour | symphonia | 78.676 [78.383, 81.892] | 82.698 [80.922, 89.694] | 0.061 | 3464 | 2160045 | 10060821 |
| six-hour | unlace | 233.524 [221.614, 330.520] | 231.951 [227.458, 447.181] | 0.039 | 3188 | 24 | 10399259 |
| synthetic-mixed | matroska-demuxer | 0.114 [0.109, 0.154] | 0.258 [0.254, 0.270] | 0.088 | 3232 | 91 | 623701 |
| synthetic-mixed | symphonia | 0.100 [0.095, 0.118] | 0.242 [0.227, 0.354] | 0.089 | 3436 | 170 | 7953 |
| synthetic-mixed | unlace | 0.051 [0.049, 0.065] | 0.216 [0.193, 0.341] | 0.039 | 3200 | 26 | 7561 |

### Same payload consumer with a common seek-aware cache

The standard BufReader variant produces about 148.33 billion backing-read bytes
for matroska-demuxer on a 10.4 MB file. This is a measured adapter/library interaction,
not physical disk traffic or an intrinsic library speed ranking. To avoid judging
it solely through that buffering combination, the additional configuration gives
all three Rust backends the same original 64 KiB positional read cache. Logical
seeks retain cached bytes; cache misses use FileExt::read_at. Symphonia retains its
own internal ring as well. Actual backing bytes and logical seek requests remain
separate; no claim of equal total internal buffering follows.

All nine common-cache cases pass a fresh full payload comparison before 126 new
time/RSS and 27 allocation trials. These are separate immutable records; the
baseline remains visible. Intervals are seven runs with the same cache conditions.

| Source | Adapter | Warm total ms | Advisory-evicted total ms | First ms | RSS KiB | Allocation requests | Read bytes |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| mono | matroska-demuxer | 0.066 [0.056, 0.071] | 0.249 [0.219, 0.384] | 0.050 | 3420 | 72 | 11009 |
| mono | symphonia | 0.105 [0.099, 0.132] | 0.268 [0.252, 6.985] | 0.089 | 3528 | 196 | 11009 |
| mono | unlace | 0.060 [0.055, 0.063] | 0.221 [0.209, 0.303] | 0.038 | 3188 | 25 | 11009 |
| six-hour | matroska-demuxer | 123.847 [122.179, 126.043] | 130.456 [128.856, 197.696] | 9.432 | 5528 | 43265 | 20927352 |
| six-hour | symphonia | 77.905 [76.740, 81.547] | 85.487 [78.666, 102.540] | 0.072 | 3432 | 2160046 | 10092565 |
| six-hour | unlace | 241.707 [237.554, 265.689] | 238.831 [232.659, 253.960] | 0.039 | 3324 | 24 | 10399259 |
| synthetic-mixed | matroska-demuxer | 0.058 [0.053, 0.069] | 0.218 [0.195, 0.231] | 0.050 | 3376 | 91 | 7561 |
| synthetic-mixed | symphonia | 0.096 [0.091, 0.105] | 0.239 [0.216, 0.257] | 0.086 | 3524 | 171 | 7561 |
| synthetic-mixed | unlace | 0.052 [0.045, 0.088] | 0.211 [0.193, 5.831] | 0.040 | 3236 | 26 | 7561 |

## Fresh-handle seek timing and I/O

All requests are nanoseconds in the exact presentation timeline. Tables display
seconds for target and actual start. The audited unlace adapter retains the
historical 64 KiB positional cache and explicit 500,000-call budget. Classic
libwebm includes Segment::Load; Nestegg and FFmpeg keep their qualified native
seek policies. Incremental libwebm has no interchangeable high-level seek policy
in the tested adapter. Rust counterpart strict seeks remain unqualified.

Warm medians below separate preparation, the seek operation and total window
work; ranges show total variability. The following table records advisory eviction
and backing reads independently. These fresh handles are not repeated seeking
within an already prepared reader.

| Source | Target s | Adapter | Prepare ms | Seek ms | Warm total ms | Actual first s | RSS KiB | Allocation requests |
| --- | ---: | --- | ---: | ---: | --- | ---: | ---: | ---: |
| mono | 0 | FFmpeg libavformat | 0.052 | 0.012 | 0.086 [0.060, 0.146] | -0.006500 | 3264 | 476 |
| mono | 0.5 | FFmpeg libavformat | 0.051 | 0.017 | 0.084 [0.057, 0.088] | 0.414500 | 3264 | 539 |
| mono | 0 | libwebm classic | 0.026 | 0.003 | 0.046 [0.043, 0.062] | -0.006500 | 4324 | 141 |
| mono | 0.5 | libwebm classic | 0.027 | 0.003 | 0.043 [0.039, 0.058] | -0.006500 | 4320 | 141 |
| mono | 0 | nestegg | 0.017 | 0.002 | 0.032 [0.030, 0.042] | -0.006500 | 2072 | 281 |
| mono | 0.5 | nestegg | 0.017 | 0.002 | 0.030 [0.026, 0.038] | -0.006500 | 2072 | 281 |
| mono | 0 | unlace | 0.047 | 0.003 | 0.078 [0.070, 0.087] | -0.006500 | 2548 | 35 |
| mono | 0.5 | unlace | 0.044 | 0.006 | 0.072 [0.061, 0.105] | -0.006500 | 2560 | 37 |
| six-hour | 0 | FFmpeg libavformat | 0.055 | 0.012 | 0.085 [0.061, 0.109] | -0.006500 | 3264 | 438 |
| six-hour | 10800 | FFmpeg libavformat | 0.057 | 98.362 | 98.442 [95.476, 193.327] | 10799.913500 | 4352 | 3780679 |
| six-hour | 21599 | FFmpeg libavformat | 0.057 | 198.500 | 198.587 [192.788, 238.921] | 21598.913500 | 4416 | 7560324 |
| six-hour | 3600 | FFmpeg libavformat | 0.058 | 33.753 | 33.821 [32.695, 64.186] | 3599.913500 | 4416 | 1260679 |
| six-hour | 0 | libwebm classic | 3.121 | 0.007 | 3.134 [3.042, 3.203] | -0.006500 | 6244 | 21731 |
| six-hour | 10800 | libwebm classic | 3.383 | 0.033 | 3.433 [3.176, 6.052] | 10798.993500 | 6244 | 21835 |
| six-hour | 21599 | libwebm classic | 3.180 | 0.027 | 3.223 [3.099, 3.696] | 21597.993500 | 6244 | 21831 |
| six-hour | 3600 | libwebm classic | 3.129 | 0.025 | 3.168 [3.127, 5.890] | 3598.993500 | 6244 | 21835 |
| six-hour | 0 | unlace | 3.959 | 2.577 | 6.568 [6.374, 10.364] | -0.006500 | 6392 | 65 |
| six-hour | 10800 | unlace | 3.856 | 0.072 | 3.959 [3.872, 6.988] | 10798.993500 | 6400 | 64 |
| six-hour | 21599 | unlace | 4.232 | 0.081 | 4.378 [3.990, 6.909] | 21597.993500 | 6272 | 64 |
| six-hour | 3600 | unlace | 4.019 | 0.067 | 4.120 [4.014, 6.556] | 3598.993500 | 6516 | 64 |

| Source | Target s | Adapter | Advisory-evicted total ms | Warm read bytes prepare / seek / window |
| --- | ---: | --- | --- | --- |
| mono | 0 | FFmpeg libavformat | 0.255 [0.247, 0.496] | 11009 / 0 / 0 |
| mono | 0.5 | FFmpeg libavformat | 0.255 [0.234, 3.442] | 11009 / 0 / 0 |
| mono | 0 | libwebm classic | 0.206 [0.201, 0.231] | 11009 / 0 / 0 |
| mono | 0.5 | libwebm classic | 0.210 [0.193, 3.859] | 11009 / 0 / 0 |
| mono | 0 | nestegg | 0.193 [0.191, 2.111] | 11009 / 0 / 0 |
| mono | 0.5 | nestegg | 0.191 [0.186, 0.204] | 11009 / 0 / 0 |
| mono | 0 | unlace | 0.232 [0.223, 0.268] | 11009 / 0 / 0 |
| mono | 0.5 | unlace | 0.237 [0.228, 0.247] | 11009 / 0 / 0 |
| six-hour | 0 | FFmpeg libavformat | 0.390 [0.377, 0.410] | 65536 / 0 / 0 |
| six-hour | 10800 | FFmpeg libavformat | 102.106 [97.818, 112.761] | 65536 / 4915200 / 0 |
| six-hour | 21599 | FFmpeg libavformat | 200.662 [194.900, 218.213] | 65536 / 9961472 / 22 |
| six-hour | 3600 | FFmpeg libavformat | 34.161 [32.913, 35.917] | 65536 / 1638400 / 0 |
| six-hour | 0 | libwebm classic | 9.356 [9.115, 12.686] | 9961493 / 65536 / 0 |
| six-hour | 10800 | libwebm classic | 9.634 [9.211, 13.139] | 9961493 / 589824 / 0 |
| six-hour | 21599 | libwebm classic | 9.408 [9.194, 16.098] | 9961493 / 589824 / 0 |
| six-hour | 3600 | libwebm classic | 9.910 [9.158, 46.543] | 9961493 / 524288 / 0 |
| six-hour | 0 | unlace | 13.727 [13.647, 14.615] | 10398147 / 9961472 / 65536 |
| six-hour | 10800 | unlace | 11.178 [11.043, 89.695] | 10398147 / 65536 / 65536 |
| six-hour | 21599 | unlace | 11.370 [11.067, 13.556] | 10398147 / 65536 / 437621 |
| six-hour | 3600 | unlace | 11.339 [11.103, 14.549] | 10398147 / 65536 / 65536 |

## Writer layout cost

These are seven counting-sink repetitions, with separate three-run allocation
medians. First emission is the first media Cluster, not acceptance of a buffered
packet; buffering trades latency control for packed output. Calls include headers,
media, indexes and patches. No device flushing or durability claim is measured.

| Mode/layout | Total ms | Finish ms | First media ms | Clusters | Output bytes | Write calls | RSS KiB | Allocation requests |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| indexed-direct | 74.237 [73.691, 74.945] | 0.892 | 0.008 | 1080001 | 22011075 | 3261608 | 2956 | 38 |
| indexed-packed | 67.445 [67.106, 69.134] | 0.896 | 0.013 | 21601 | 11215581 | 64807 | 2892 | 44 |
| live-direct | 70.052 [68.976, 127.775] | 0.000 | 0.008 | 1080001 | 20758082 | 3240004 | 2352 | 24 |
| live-packed | 63.972 [62.953, 68.056] | 0.000 | 0.012 | 21601 | 9962588 | 43203 | 2332 | 30 |

## Interpretation and unfavorable cases

The strict six-hour total remains unfavorable for unlace: about 253 ms, against
63 ms Nestegg, 97 ms incremental libwebm, 150 ms classic libwebm and 230 ms FFmpeg.
Its first packet is about 0.037 ms versus classic libwebm's 3.19 ms, while Nestegg
is faster still at about 0.020 ms. Unlace's low allocation request count and small
process RSS in that job do not imply throughput leadership.

For the six-hour payload-only job with standard buffering, Symphonia is about
79 ms, unlace 234 ms and matroska-demuxer 2,669 ms. With the common seek-aware
cache, the same consumer is about 78 ms, 242 ms and 124 ms respectively. Unlace
is therefore slower than both other Rust readers in the common-cache long-file
configuration; its apparent win over matroska-demuxer in the baseline must not
be advertised as an intrinsic library advantage. The changed cache also reduces
that adapter's backing reads from about 148.33 GB to 20.93 MB. These measurements
do not qualify full metadata replacement. Symphonia's roughly 2.16 million
allocation requests versus unlace's 24 show a different cost tradeoff, not why
one is faster.
No causal bottleneck attribution is made without profiling. Short-file totals
have large relative startup/scheduling variability; retain their full ranges.

The unlace sparse positional cache still overreads during opening/indexing; its
small index destination reads are not the same as small total preparation I/O.
The explicit before-first-cue budget remains unfavorable and is not silently
raised in the library defaults. Nestegg's trailing-index seek admission failures
remain outside equivalent timing, as in the historical report. Native End and
these consumer gates do not establish equal validation/completeness guarantees.

Packing reduces this writer experiment from 1,080,001 to 21,601 Clusters and
substantially fewer Write calls. Its additional bounded buffering/allocation and
later first media emission remain visible. A counting sink cannot establish
filesystem, network or real-media muxer performance. No overall best-library,
production-readiness or cross-platform claim follows from these numbers.

## Artifact identities and reproduction

Current adapter executable SHA-256 values follow. Native executable identities
are unchanged from the historical report; environment.json records and verifies
all of them before/after measurement. Complete source/corpus/lockfile identities,
qualification manifests, every raw trial and derived medians remain ignored under
`runs/release-audit/performance`.

| Adapter | SHA-256 |
| --- | --- |
| strict-reader | `f53638955ebb5a20c67ca4b24a0dd1736348ae002c998765ea6b4a67656231a8` |
| strict-seek | `69844b2c86206b606575d383f526702021e3b104e6a1b6a5cf0420e96c8411b1` |
| payload | `69d1ef3f0759ed90eed1417224956c733b6a6437db7509373252cbbb83ef217e` |
| writer | `cd2390e982e21088119158fda9fdf01a8f094bc00651ac32a0ed1db281b556c5` |

Common-cache adapter SHA-256: `2188437dc2722fcd74924c6ea4a3c1f4703a2ac16e06ebafdd8bf2df89f5aed3`.

The isolated research Cargo.lock SHA-256 is `7ebc1a8f2ed43e56b1d4313c87a8eb2ff7841eb4bfcf5db0a18ea95427b6f309`.
Its external dependencies are confined to that ignored research workspace;
ordinary unlace Cargo metadata still has zero dependencies. Reproduction uses
Rust 1.89.0 release builds of the original adapter sources, immutable native
executables and source corpus, then runs local `qualify.py`, `measure.py` and
`analyze.py`. The qualification requires complete full-field/payload manifests
before timing. The analysis verifies the original 826/183 trial sets and pins; cached_run.py
separately qualifies and verifies the additional 126/27 sets. Run write_report.py
and write_cached_report.py in that order to regenerate the combined report.

The three source files in the payload-only cohort retain the historical identities:
mono `12c21d56900699e4e3c9fcde535dd8003c4f9555930a8e5f1a3dabd1ee8fd630`,
mixed `4a6d0991e7ca554a9b822495c20da9168be8c53f4069a1d1b7d55c6f1d6c84ba`, and the
six-hour source is `689a57fa5aa23bf9a7be2882b4d4c70f8d79ed6209aafa5c94cf384014e26c2f`.
No raw media or benchmark logs are uploaded with the source report.
