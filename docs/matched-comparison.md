# Matched selected-audio comparison

Measured locally on 2026-11-01 against unlace source
`2c0bc898ccd8637f9cbe644b0eb50898e196f5d5` (unpublished Cargo 0.1.0).
This report measures a defined packet-consumer job. It does not establish an
overall library ranking, general Matroska compatibility or equal validation
and completion guarantees. At the measured revision, writing and broader
decoded-audio qualification remained unfinished. Raw sources, licensed media, adapters and process records
stay local under the repository's local-only policy.

## Revisions and build

All five required competitors were available and built. No unavailable library
is reported as a failed parser. Both libwebm APIs were studied separately.
Upstream source and licenses remained unchanged; the adapters were independently
written against public APIs.

| Implementation | Pinned source or artifact | Build |
| --- | --- | --- |
| unlace | `2c0bc898ccd8637f9cbe644b0eb50898e196f5d5` | Rust 1.89.0, release |
| [libwebm](https://github.com/webmproject/libwebm/tree/a7c6662c68a90af18dd53dbb1f4262a346e9a547) | `a7c6662c68a90af18dd53dbb1f4262a346e9a547` | GCC 15.2.0, C++11, `-O3` |
| [Nestegg](https://github.com/mozilla/nestegg/tree/767aab25013acefbdcc6d68a2a7a2c9081303e3f) | `767aab25013acefbdcc6d68a2a7a2c9081303e3f` | GCC 15.2.0, C11, `-O3` |
| [libavformat](https://ffmpeg.org/releases/ffmpeg-7.0.2.tar.xz) | FFmpeg 7.0.2, libavformat 61.1.100; release tar SHA-256 `8646515b638a3ad303e23af6a3587734447cb8fc0a0c064ecdb8e95c4fd8b389` | Static libraries, configuration below |
| [Symphonia](https://github.com/pdeljanov/Symphonia/tree/ee35874b571a35a9a6e15d3bc9a3aaf8f11fbeee) | core/format-mkv/common/metadata crates 0.6.1, VCS `ee35874b571a35a9a6e15d3bc9a3aaf8f11fbeee` | Rust 1.89.0, release |
| [matroska-demuxer](https://github.com/hasenbanck/matroska-demuxer/tree/e038e97b659ec2750b045a5f7a42e60b73c9491f) | 0.8.1 at `e038e97b659ec2750b045a5f7a42e60b73c9491f` | Rust 1.89.0, release |

The Rust adapters use an isolated ignored Cargo workspace; they do not add
unlace dependencies. Its lockfile SHA-256 is
`1687effdff34e9f30cc7a7c4e2cce128492524a6bfbfc06085dd025150c9737b`.
Registry dependencies are pinned there: autocfg 1.5.1, bitflags 2.13.2,
bytemuck 1.25.2, lazy_static 1.5.1, log 0.4.35, num-complex 0.4.6,
num-traits 0.2.19, regex-lite 0.1.9, smallvec 1.16.4 and the four Symphonia
0.6.1 crates. Core and format-mkv archive SHA-256 values are respectively
`01c412864d599d4750d0c3d684d7e093ec05e5309681ef5252cc1096a437f6e0`
and `d015c5c0558864665894b3f4cbd95e10abb01b9c868e751c72670f326a56360e`.
The source identities are independent of the current upstream branches.

Rust uses default release optimization (level 3, no LTO), LLVM 20.1.7 and
the default x86_64 target, without `target-cpu=native`. The C/C++ compiler
is Ubuntu GCC 15.2.0-16ubuntu1. FFmpeg uses its default optimized configuration:

```sh
./configure --disable-everything --enable-demuxer=matroska \
  --enable-protocol=file --enable-parser=opus,vp9,vorbis \
  --disable-programs --disable-doc --disable-network --disable-autodetect \
  --disable-x86asm --disable-shared --enable-static --cc=cc
make -j4 libavformat/libavformat.a libavcodec/libavcodec.a libavutil/libavutil.a
```

The adapter links those libraries with `-lm -pthread`, opens the explicit
Matroska demuxer over custom AVIO and does not call `avformat_find_stream_info`
or instantiate a decoder. Symphonia uses `MkvReader::try_new` directly.
Libwebm classic calls `Segment::Load`; its directory retention and startup
are part of that variant. Incremental libwebm uses `WebmParser::Feed` callbacks,
a reusable contiguous pending-block buffer and at most 256 frame descriptors.
It waits for BlockGroup end before exposing late padding. Treating classic
full-load results as the only libwebm implementation would be incorrect.

## Corpus and correctness gate

The nine producer/long sources are the two original mono/stereo Opus samples,
two native Chromium MediaRecorder streams (audio-only and VP9/audio), completed
Big Buck Bunny YouTube Opus, original FFmpeg Opus, mixed VP9/Opus and live
Vorbis samples, and the original six-hour Opus container. Rights, full hashes,
producer pins and independent checks are retained in
[packet qualification](packet-qualification.md), [Opus timing](opus-timing.md)
and [robustness qualification](robustness.md). Big Buck Bunny is Blender's
CC BY 3.0 material. Operational downloads and signed URLs are not included here.

Sixteen further original containers use our own three-byte encoded silence
packet `f8fffe`: no/fixed/Xiph/EBML lacing, zero and both signs of padding,
negative raw time, the minimum signed block timecode, disabled-first dual audio,
and preroll with one extra nanosecond. A single-frame padding case uses 10 ms;
three-frame groups use 25 ms across a 60 ms block. The generated arithmetic
and exact bytes are known independently of any demuxer.

Full bounded JSONL manifests compare every selected payload byte and boundary,
order, selected-track identity, CodecPrivate, exact presentation nanoseconds,
raw CodecDelay/SeekPreRoll and signed whole-block DiscardPadding. Missing padding
is unavailable, never silently zero. First-audio stream indexes include video
tracks: index 1 maps to TrackNumber 2 in the two mixed sources. The disabled-first
fixture requires the second audio track. Identical silence payloads alone cannot
prove that selection. Native failure to obtain final End excludes a row even
when packets were emitted. No manifest serialization time enters the benchmark.

This gate is the measured consumer contract, not equivalence of every public
packet field. Native derived packet duration is distinct from raw BlockDuration;
flags, original offsets, arbitrary codec transformations, complete index
validation and unlace's finite/suffix completion evidence are not timing-gated
as interchangeable APIs. Native EOF is never promoted to unlace completeness.

| Adapter | Exact consumer gate | Native completion failures | Payload differences on compared successful rows |
| --- | --- | --- | --- |
| matroska-demuxer | 0/25 | 3 | 0 |
| Symphonia | 0/25 | 3 | 0 |
| libwebm classic full-load | 24/25 | 0 | 0 |
| libwebm incremental | 25/25 | 0 | 0 |
| Nestegg | 22/25 | 0 | 0 |
| libavformat | 20/25 | 0 | 0 |

The fractions count cases in this corpus; they are not quality scores.
A failed selection can still have identical payload bytes in a synthetic source.
The strict gate excludes useful readers when their public metadata cannot
represent this particular raw contract.

- Matroska-demuxer's public Frame lacks padding. Its laced frame timestamps repeat
  the block timestamp on the controlled fixture instead of adding DefaultDuration.
  Both browser inputs return `CantFindCluster`; the minimum relative-time case
  returns `TryFromIntError` in the optimized build. Negative raw time becomes zero.
- Symphonia emits the real browser packets but ends with `UnexpectedEof`, also
  observed on the live Vorbis source. Raw delay/preroll are unavailable in this
  adapter, Opus padding is absent from the emitted trim fields, and Vorbis extra
  data normalizes initialization by omitting the comment header. That is different
  from preserving the original CodecPrivate, not a claim of invalid decoder setup.
- Classic libwebm exposes raw relative and Cluster timecodes. The adapter computes
  `(cluster + relative) * scale - delay` in i128, then adds known DefaultDuration
  for later laces. Its convenience nanosecond getter returns `-1` on the negative
  raw-time fixture; public raw accessors recover the exact value. The first-enabled
  audio policy is unavailable in the tested classic adapter, so disabled-first
  selection fails. Incremental libwebm exposes the enabled flag and passes all 25.
- Nestegg preserves the tested raw delay/preroll/padding. Its negative raw-time
  results become zero, and the tested first-audio adapter cannot implement the
  enabled-track selection policy. Other selected bytes/times/trim match.
- Libavformat uses millisecond packet timestamps on the Opus cohort. For a 6.5 ms
  delay its native timestamps differ from exact presentation by 0.5 ms. The adapter
  reconstructs exact values only after proving that public initial-padding samples
  represent the source's raw delay exactly. With `q` nanoseconds per tick and `d`
  exact delay, it uses `pts*q + round_nearest_positive(d/q)*q - d`, matching the
  pinned source's subtraction rule. It does not apply the delay twice. Public
  sample padding is converted with rational arithmetic. The YouTube final stored
  19,291,667 ns becomes 57,875,000/3 ns from samples, so that source is excluded
  from exact-metadata timing. One extra preroll nanosecond is also lost. Negative
  raw timestamps and disabled-first selection fail this adapter's gate.

No Rust competitor timing is presented as an equivalent replacement after
failing these fields. Their sequential correctness adapters were tested; their
strict-contract timing and seek adapters remain unqualified rather than slow,
unavailable or untested parsers.

## Measurement method

Hardware: AMD Ryzen AI 9 HX 370, 24 logical CPUs; child affinity is logical CPU 0.
OS: x86_64 Linux 7.0.0-38-generic, glibc 2.43. Clock frequency and desktop background
activity are uncontrolled. This is one machine and seven samples per condition,
not a cross-platform or statistically general performance result.

Each timed child opens a fresh handle. A monotonic timer starts inside main
before opening the file and stops after native iteration End or the requested
window, before library/context destruction and output formatting. First-packet
latency stops when the first selected payload and its group metadata are ready,
before hashing that payload. Open/prepare time is recorded separately. Steady
iteration below is elapsed minus first-packet time; it includes the first packet's
consumer work. Whole-process supervisor wall time is retained locally and includes
process/loader/cleanup overhead. It is not substituted for the inner timer.

Every adapter uses a 65,536-byte input buffer/cache, retaining the library's own
buffers. C++ callbacks reuse their pending block storage. Sequential Rust uses
BufReader; C/C++ positional callbacks retain a pread cache; AVIO has its own 64 KiB
buffer. Different skip/seek patterns can reread or avoid different ranges. These
are explicit adapter variants, not identical internal I/O algorithms. Read bytes
below count actual backing reads, including prefetch/repeats, not unique bytes,
logical parser requests or network transfers. The C Nestegg cache is on the stack;
Rust/C++/AVIO caches are on the heap. Process allocation totals include that choice.

All consumers run the same FNV-1a-64 update (offset 14695981039346656037, prime
1099511628211, wrapping u64). Initialization hashes the little-endian u64 private
length, private bytes, u64 delay and preroll. Each packet hashes u64 payload length,
original bytes, signed i128 presentation ns and signed i64 whole-block padding.
A full independent manifest equality check precedes this inexpensive digest;
matching FNV alone is not proof. Every timed and allocation-instrumented result
must match its reference count, payload total and digest.

For warm trials the supervisor prereads the input in 1 MiB chunks. For advisory
trials it requests per-file `POSIX_FADV_DONTNEED`. Neither flushes global caches;
physical-device cold reads are unproven. Child runs are sequential in randomized
order with seed 577113, seven repetitions per condition. Limits are 1 GiB virtual
address space, 30 seconds CPU and a 45-second supervisor timeout. Time/RSS trials
have no allocation interposer. GNU time reports process peak RSS including runtime,
stack, library code and adapter buffers; it is not pure parser heap usage.

Three separate warm allocation runs use a local Linux/glibc C interposer for
malloc/calloc/realloc/memalign/aligned_alloc/posix_memalign. It counts whole-process
requests and requested bytes, including startup, adapters, output and cleanup;
realloc counts its full requested size. It does not measure live heap or internal
allocator activity. A controlled six-call test verifies an 824-byte request delta.
Instrumented elapsed time is discarded. Neither the probe nor competitor Cargo
packages become an unlace build dependency.

The six-hour file is 10,399,259 bytes of repeated encoded silence and 1,080,001
packets, not six hours of varied speech or a multi-gigabyte source. Short, browser
mixed, lacing/late-padding and negative-time inputs remain visible below.

### Warm sequential timing

Milliseconds are median [minimum, maximum], seven runs. Throughput uses the
whole input size divided by median elapsed time; packet hashing is included.

| Source | Adapter | First ms | Total ms | Steady ms | MiB/s |
| --- | --- | --- | --- | --- | --- |
| mono | unlace | 0.038 [0.031, 0.058] | 0.060 [0.054, 0.081] | 0.023 | 174.72 |
| mono | libwebm classic full-load | 0.036 [0.028, 0.044] | 0.052 [0.044, 0.061] | 0.016 | 202.81 |
| mono | libwebm incremental | 0.086 [0.054, 0.102] | 0.111 [0.072, 0.122] | 0.020 | 94.88 |
| mono | Nestegg | 0.020 [0.018, 0.025] | 0.033 [0.030, 0.037] | 0.012 | 322.94 |
| mono | libavformat | 0.091 [0.054, 0.115] | 0.114 [0.077, 0.149] | 0.023 | 92.44 |
| six-hour | unlace | 0.039 [0.036, 0.065] | 254.051 [244.167, 257.778] | 253.986 | 39.04 |
| six-hour | libwebm classic full-load | 3.169 [3.125, 3.835] | 148.354 [147.001, 163.273] | 145.185 | 66.85 |
| six-hour | libwebm incremental | 0.062 [0.053, 0.086] | 96.144 [95.775, 97.378] | 96.088 | 103.15 |
| six-hour | Nestegg | 0.020 [0.018, 0.021] | 62.256 [61.483, 63.131] | 62.235 | 159.30 |
| six-hour | libavformat | 0.069 [0.057, 0.073] | 227.632 [226.572, 228.913] | 227.563 | 43.57 |
| browser-mixed | unlace | 0.037 [0.033, 0.053] | 0.071 [0.066, 0.087] | 0.033 | 300.64 |
| browser-mixed | libwebm classic full-load | 0.044 [0.043, 0.065] | 0.061 [0.060, 0.082] | 0.017 | 346.64 |
| browser-mixed | libwebm incremental | 0.095 [0.056, 0.127] | 0.125 [0.085, 0.168] | 0.029 | 169.92 |
| browser-mixed | Nestegg | 0.019 [0.016, 0.020] | 0.041 [0.038, 0.041] | 0.022 | 525.22 |
| browser-mixed | libavformat | 0.076 [0.061, 0.115] | 0.108 [0.093, 0.148] | 0.032 | 196.34 |
| fixed-padding-25000000 | unlace | 0.038 [0.033, 0.055] | 0.039 [0.034, 0.056] | 0.001 | 5.54 |
| fixed-padding-25000000 | libwebm classic full-load | 0.025 [0.020, 0.029] | 0.025 [0.020, 0.030] | 0.001 | 8.58 |
| fixed-padding-25000000 | libwebm incremental | 0.057 [0.055, 0.090] | 0.058 [0.055, 0.091] | 0.001 | 3.75 |
| fixed-padding-25000000 | Nestegg | 0.022 [0.015, 0.042] | 0.022 [0.015, 0.042] | 0.000 | 9.80 |
| fixed-padding-25000000 | libavformat | 0.061 [0.050, 0.071] | 0.062 [0.051, 0.073] | 0.002 | 3.47 |
| none-padding-10000000 | unlace | 0.033 [0.031, 0.036] | 0.033 [0.032, 0.037] | 0.001 | 6.33 |
| none-padding-10000000 | libwebm classic full-load | 0.027 [0.023, 0.049] | 0.028 [0.023, 0.050] | 0.001 | 7.56 |
| none-padding-10000000 | libwebm incremental | 0.058 [0.048, 0.090] | 0.058 [0.049, 0.091] | 0.001 | 3.59 |
| none-padding-10000000 | Nestegg | 0.016 [0.014, 0.021] | 0.016 [0.014, 0.021] | 0.000 | 12.92 |
| none-padding-10000000 | libavformat | 0.057 [0.043, 0.087] | 0.058 [0.044, 0.088] | 0.001 | 3.62 |
| negative-raw-time | unlace | 0.037 [0.034, 0.052] | 0.038 [0.035, 0.052] | 0.001 | 5.15 |
| negative-raw-time | libwebm classic full-load | 0.026 [0.020, 0.041] | 0.027 [0.020, 0.042] | 0.000 | 7.31 |
| negative-raw-time | libwebm incremental | 0.055 [0.052, 0.060] | 0.055 [0.053, 0.060] | 0.000 | 3.52 |

### Advisory-evicted sequential timing

Milliseconds are median [minimum, maximum], seven runs. Throughput uses the
whole input size divided by median elapsed time; packet hashing is included.

| Source | Adapter | First ms | Total ms | Steady ms | MiB/s |
| --- | --- | --- | --- | --- | --- |
| mono | unlace | 0.226 [0.189, 0.495] | 0.248 [0.212, 0.518] | 0.022 | 42.35 |
| mono | libwebm classic full-load | 0.189 [0.182, 0.191] | 0.205 [0.198, 0.209] | 0.016 | 51.30 |
| mono | libwebm incremental | 0.226 [0.210, 0.268] | 0.243 [0.228, 0.285] | 0.017 | 43.16 |
| mono | Nestegg | 0.189 [0.169, 0.385] | 0.201 [0.181, 0.397] | 0.012 | 52.18 |
| mono | libavformat | 0.222 [0.207, 0.233] | 0.244 [0.230, 0.278] | 0.022 | 42.98 |
| six-hour | unlace | 0.403 [0.394, 0.442] | 253.374 [245.484, 260.221] | 252.948 | 39.14 |
| six-hour | libwebm classic full-load | 9.246 [8.980, 71.369] | 155.025 [153.886, 216.648] | 145.279 | 63.97 |
| six-hour | libwebm incremental | 0.434 [0.417, 3.542] | 98.819 [98.144, 135.656] | 98.300 | 100.36 |
| six-hour | Nestegg | 0.400 [0.377, 0.489] | 64.753 [64.286, 65.920] | 64.350 | 153.16 |
| six-hour | libavformat | 0.447 [0.423, 0.505] | 232.517 [228.849, 246.320] | 232.070 | 42.65 |
| browser-mixed | unlace | 0.227 [0.217, 0.374] | 0.260 [0.250, 0.408] | 0.033 | 81.79 |
| browser-mixed | libwebm classic full-load | 0.217 [0.212, 0.328] | 0.234 [0.229, 0.345] | 0.017 | 90.81 |
| browser-mixed | libwebm incremental | 0.234 [0.217, 0.280] | 0.263 [0.246, 0.310] | 0.029 | 80.91 |
| browser-mixed | Nestegg | 0.193 [0.177, 1.216] | 0.215 [0.198, 1.237] | 0.022 | 99.24 |
| browser-mixed | libavformat | 0.262 [0.225, 0.332] | 0.295 [0.258, 0.365] | 0.032 | 72.30 |
| fixed-padding-25000000 | unlace | 0.179 [0.174, 0.336] | 0.180 [0.175, 0.337] | 0.001 | 1.21 |
| fixed-padding-25000000 | libwebm classic full-load | 0.171 [0.163, 0.184] | 0.172 [0.163, 0.185] | 0.000 | 1.26 |
| fixed-padding-25000000 | libwebm incremental | 0.247 [0.192, 0.370] | 0.248 [0.192, 0.371] | 0.001 | 0.87 |
| fixed-padding-25000000 | Nestegg | 0.158 [0.151, 5.028] | 0.158 [0.151, 5.029] | 0.000 | 1.37 |
| fixed-padding-25000000 | libavformat | 0.231 [0.195, 0.359] | 0.232 [0.196, 0.360] | 0.002 | 0.93 |
| none-padding-10000000 | unlace | 0.187 [0.177, 2.977] | 0.188 [0.178, 2.978] | 0.001 | 1.12 |
| none-padding-10000000 | libwebm classic full-load | 0.175 [0.168, 0.203] | 0.175 [0.168, 0.203] | 0.001 | 1.20 |
| none-padding-10000000 | libwebm incremental | 0.240 [0.201, 3.359] | 0.241 [0.202, 3.360] | 0.001 | 0.87 |
| none-padding-10000000 | Nestegg | 0.166 [0.159, 0.228] | 0.166 [0.159, 0.228] | 0.000 | 1.26 |
| none-padding-10000000 | libavformat | 0.206 [0.181, 0.292] | 0.207 [0.183, 0.293] | 0.001 | 1.01 |
| negative-raw-time | unlace | 0.177 [0.176, 0.203] | 0.177 [0.176, 0.203] | 0.001 | 1.10 |
| negative-raw-time | libwebm classic full-load | 0.171 [0.157, 0.192] | 0.171 [0.158, 0.193] | 0.000 | 1.14 |
| negative-raw-time | libwebm incremental | 0.198 [0.185, 0.335] | 0.198 [0.185, 0.336] | 0.000 | 0.98 |

### Sequential resources

Peak RSS is the warm median [minimum, maximum] in KiB. Allocation counts/bytes
show the minimum and maximum across three separate instrumented processes.

| Source | Adapter | RSS KiB | Allocation requests | Requested bytes | Backing read bytes |
| --- | --- | --- | --- | --- | --- |
| mono | unlace | 2528 [2384, 2632] | 24 | 70855 | 11,009 |
| mono | libwebm classic full-load | 4352 [4224, 4352] | 141 | 174897 | 11,009 |
| mono | libwebm incremental | 4524 [4444, 4564] | 468 | 178919 | 11,009 |
| mono | Nestegg | 1948 [1944, 2076] | 268 | 31752 | 11,009 |
| mono | libavformat | 3140 [3012, 3296] | 457 | 120266 | 11,009 |
| six-hour | unlace | 2540 [2512, 2592] | 23 | 70085 | 10,399,259 |
| six-hour | libwebm classic full-load | 236096 [235964, 236100] | 2203226 | 282842805 | 27,262,997 |
| six-hour | libwebm incremental | 4524 [4464, 4680] | 1253205 | 98496271 | 10,399,259 |
| six-hour | Nestegg | 1948 [1884, 2076] | 4320029 | 141494402 | 10,399,259 |
| six-hour | libavformat | 4292 [4228, 4356] | 7560169 | 466021482 | 10,027,030 |
| browser-mixed | unlace | 2536 [2516, 2584] | 23 | 70821 | 22,330 |
| browser-mixed | libwebm classic full-load | 4352 [4296, 4360] | 131 | 248866 | 22,330 |
| browser-mixed | libwebm incremental | 4524 [4408, 4672] | 539 | 181425 | 22,330 |
| browser-mixed | Nestegg | 1948 [1944, 2076] | 248 | 43272 | 22,330 |
| browser-mixed | libavformat | 3196 [3008, 3204] | 488 | 132975 | 22,330 |
| fixed-padding-25000000 | unlace | 2536 [2512, 2576] | 22 | 70023 | 227 |
| fixed-padding-25000000 | libwebm classic full-load | 4352 [4216, 4360] | 21 | 168825 | 227 |
| fixed-padding-25000000 | libwebm incremental | 4524 [4452, 4536] | 406 | 173014 | 227 |
| fixed-padding-25000000 | Nestegg | 1948 [1884, 2076] | 33 | 14472 | 227 |
| fixed-padding-25000000 | libavformat | 3076 [3012, 3204] | 62 | 78903 | 227 |
| none-padding-10000000 | unlace | 2576 [2340, 2636] | 22 | 70016 | 220 |
| none-padding-10000000 | libwebm classic full-load | 4348 [4212, 4352] | 21 | 168793 | 220 |
| none-padding-10000000 | libwebm incremental | 4524 [4496, 4692] | 404 | 172980 | 220 |
| none-padding-10000000 | Nestegg | 1948 [1756, 2076] | 29 | 14402 | 220 |
| none-padding-10000000 | libavformat | 3204 [3076, 3296] | 50 | 78156 | 220 |
| negative-raw-time | unlace | 2524 [2512, 2556] | 22 | 70012 | 204 |
| negative-raw-time | libwebm classic full-load | 4352 [4224, 4360] | 21 | 168769 | 204 |
| negative-raw-time | libwebm incremental | 4524 [4520, 4692] | 404 | 172836 | 204 |

Unlace's six-hour full iteration is slower than the qualified native competitors
in this job; low allocation count does not imply high throughput. Its early first
packet and small retained footprint compare favorably with classic full-load
libwebm on this input, while incremental libwebm avoids that directory-retention
cost and Nestegg is faster. Short-input startup is small and noisy; the reported
ranges and advisory-cache results limit any inference. No implementation is
called universally better.

## Fresh-handle seek windows

Before timing, native seek adapters must reproduce the exact one-second selected
packet window from the full manifest and begin early enough to cover
`max(source_first_time, target - 80 ms)`. This is observed encoded preroll coverage,
not decoder-state restoration. Near the beginning, there is no earlier packet to
supply. Each query opens a fresh handle: these numbers do not measure an already
prepared cursor or prove repeated-seek state correctness. Unlace's independent
forward/backward suffix state checks are described in [seeking.md](seeking.md).

Unlace uses SeekReader over a retained 64 KiB immutable-file positional cache and
an explicit 500,000-call allowance. The documented default 65,536-call refusal
before the first cue on this source remains unchanged. Classic libwebm uses
Track::Seek after Segment::Load; Nestegg uses nestegg_track_seek; FFmpeg uses
av_seek_frame with backward selection, an explicit 80 ms request and clamping to
source beginning. Returned times/trim use the same qualified normalization.

Nestegg's initialization cannot discover this fixture's trailing Cues/SeekHead.
A diagnostic attempt to read to native EOF before retrying still did not expose
Cues through its public API. Its six-hour track-seek attempts fail and are not
replaced silently with a custom EBML index parser. It qualifies on the short
indexed sample. Incremental libwebm provides a parser reset at a known element
boundary, not a complete index/track seek policy; that composed adapter is
untested here. Rust competitors remain excluded by the metadata gate.

Open time includes preparation/indexing. Seek time covers the native seek call;
total also includes recovering and hashing the window. Read volumes distinguish
preparation, seek and subsequent packet reads, including cache overfetch.

### Warm seek timing and I/O

Times are milliseconds, median [minimum, maximum], seven fresh handles per row.

| Source | Target s | Adapter | Prepare ms | Seek ms | Total ms | Read bytes prepare / seek / window |
| --- | --- | --- | --- | --- | --- | --- |
| mono | 0 | unlace | 0.046 [0.039, 0.064] | 0.002 [0.002, 0.004] | 0.075 [0.068, 0.097] | 11,009 / 0 / 0 |
| mono | 0 | libwebm classic full-load | 0.027 [0.024, 0.040] | 0.003 [0.003, 0.005] | 0.047 [0.044, 0.062] | 11,009 / 0 / 0 |
| mono | 0 | Nestegg | 0.016 [0.015, 0.024] | 0.002 [0.002, 0.002] | 0.033 [0.030, 0.042] | 11,009 / 0 / 0 |
| mono | 0 | libavformat | 0.052 [0.048, 0.055] | 0.012 [0.010, 0.013] | 0.088 [0.081, 0.089] | 11,009 / 0 / 0 |
| mono | 0.5 | unlace | 0.045 [0.041, 0.063] | 0.006 [0.006, 0.008] | 0.074 [0.070, 0.097] | 11,009 / 0 / 0 |
| mono | 0.5 | libwebm classic full-load | 0.026 [0.023, 0.038] | 0.003 [0.003, 0.005] | 0.042 [0.038, 0.056] | 11,009 / 0 / 0 |
| mono | 0.5 | Nestegg | 0.016 [0.015, 0.018] | 0.002 [0.001, 0.002] | 0.028 [0.026, 0.032] | 11,009 / 0 / 0 |
| mono | 0.5 | libavformat | 0.050 [0.046, 0.077] | 0.017 [0.014, 0.022] | 0.087 [0.076, 0.115] | 11,009 / 0 / 0 |
| six-hour | 0 | unlace | 3.869 [3.776, 4.421] | 2.496 [2.476, 2.783] | 6.394 [6.335, 6.979] | 10,398,147 / 9,961,472 / 65,536 |
| six-hour | 0 | libwebm classic full-load | 3.129 [3.107, 3.510] | 0.007 [0.007, 0.008] | 3.145 [3.123, 3.527] | 9,961,493 / 65,536 / 0 |
| six-hour | 0 | libavformat | 0.062 [0.059, 0.083] | 0.013 [0.010, 0.013] | 0.090 [0.084, 0.111] | 65,536 / 0 / 0 |
| six-hour | 10800 | unlace | 3.853 [3.790, 4.406] | 0.070 [0.067, 0.086] | 3.958 [3.896, 4.510] | 10,398,147 / 65,536 / 65,536 |
| six-hour | 10800 | libwebm classic full-load | 3.120 [3.099, 3.218] | 0.026 [0.024, 0.029] | 3.160 [3.139, 3.262] | 9,961,493 / 589,824 / 0 |
| six-hour | 10800 | libavformat | 0.058 [0.044, 0.064] | 96.711 [95.955, 100.399] | 96.791 [96.033, 100.478] | 65,536 / 4,915,200 / 0 |
| six-hour | 3600 | unlace | 3.844 [3.799, 4.617] | 0.063 [0.062, 0.068] | 3.941 [3.896, 4.717] | 10,398,147 / 65,536 / 65,536 |
| six-hour | 3600 | libwebm classic full-load | 3.178 [3.113, 3.448] | 0.026 [0.024, 0.028] | 3.218 [3.152, 3.491] | 9,961,493 / 524,288 / 0 |
| six-hour | 3600 | libavformat | 0.058 [0.041, 0.067] | 32.747 [32.130, 34.300] | 32.817 [32.190, 34.375] | 65,536 / 1,638,400 / 0 |
| six-hour | 21599 | unlace | 3.865 [3.729, 4.643] | 0.076 [0.075, 0.090] | 3.999 [3.873, 4.781] | 10,398,147 / 65,536 / 437,621 |
| six-hour | 21599 | libwebm classic full-load | 3.165 [3.109, 3.322] | 0.025 [0.022, 0.029] | 3.206 [3.147, 3.366] | 9,961,493 / 589,824 / 0 |
| six-hour | 21599 | libavformat | 0.060 [0.047, 0.072] | 196.455 [191.677, 204.083] | 196.538 [191.758, 204.156] | 65,536 / 9,961,472 / 22 |

### Advisory-evicted seek timing and I/O

Times are milliseconds, median [minimum, maximum], seven fresh handles per row.

| Source | Target s | Adapter | Prepare ms | Seek ms | Total ms | Read bytes prepare / seek / window |
| --- | --- | --- | --- | --- | --- | --- |
| mono | 0 | unlace | 0.206 [0.198, 0.218] | 0.003 [0.002, 0.003] | 0.236 [0.227, 0.248] | 11,009 / 0 / 0 |
| mono | 0 | libwebm classic full-load | 0.190 [0.185, 0.214] | 0.003 [0.003, 0.003] | 0.209 [0.204, 0.234] | 11,009 / 0 / 0 |
| mono | 0 | Nestegg | 0.181 [0.168, 0.183] | 0.002 [0.001, 0.002] | 0.196 [0.184, 0.199] | 11,009 / 0 / 0 |
| mono | 0 | libavformat | 0.220 [0.217, 0.253] | 0.012 [0.008, 0.013] | 0.254 [0.251, 0.289] | 11,009 / 0 / 0 |
| mono | 0.5 | unlace | 0.204 [0.200, 0.219] | 0.006 [0.005, 0.007] | 0.233 [0.229, 0.250] | 11,009 / 0 / 0 |
| mono | 0.5 | libwebm classic full-load | 0.190 [0.183, 0.198] | 0.003 [0.003, 0.005] | 0.205 [0.198, 0.217] | 11,009 / 0 / 0 |
| mono | 0.5 | Nestegg | 0.173 [0.164, 0.203] | 0.002 [0.001, 0.002] | 0.184 [0.176, 0.216] | 11,009 / 0 / 0 |
| mono | 0.5 | libavformat | 0.216 [0.206, 0.235] | 0.018 [0.016, 0.019] | 0.249 [0.237, 0.270] | 11,009 / 0 / 0 |
| six-hour | 0 | unlace | 11.327 [10.967, 11.596] | 2.485 [2.458, 2.557] | 13.811 [13.504, 14.166] | 10,398,147 / 9,961,472 / 65,536 |
| six-hour | 0 | libwebm classic full-load | 9.309 [9.113, 10.228] | 0.008 [0.007, 0.009] | 9.325 [9.129, 10.245] | 9,961,493 / 65,536 / 0 |
| six-hour | 0 | libavformat | 0.443 [0.413, 0.471] | 0.011 [0.008, 0.013] | 0.476 [0.435, 0.496] | 65,536 / 0 / 0 |
| six-hour | 10800 | unlace | 11.266 [11.082, 11.649] | 0.068 [0.067, 0.091] | 11.369 [11.185, 11.772] | 10,398,147 / 65,536 / 65,536 |
| six-hour | 10800 | libwebm classic full-load | 9.244 [9.168, 10.237] | 0.028 [0.027, 0.030] | 9.286 [9.211, 10.281] | 9,961,493 / 589,824 / 0 |
| six-hour | 10800 | libavformat | 0.429 [0.417, 0.450] | 97.129 [96.290, 101.043] | 97.585 [96.732, 101.514] | 65,536 / 4,915,200 / 0 |
| six-hour | 3600 | unlace | 11.282 [11.215, 11.417] | 0.062 [0.061, 0.065] | 11.379 [11.309, 11.518] | 10,398,147 / 65,536 / 65,536 |
| six-hour | 3600 | libwebm classic full-load | 9.230 [8.908, 9.922] | 0.028 [0.024, 0.032] | 9.273 [8.953, 9.966] | 9,961,493 / 524,288 / 0 |
| six-hour | 3600 | libavformat | 0.421 [0.396, 0.437] | 33.166 [32.426, 34.296] | 33.603 [32.871, 34.754] | 65,536 / 1,638,400 / 0 |
| six-hour | 21599 | unlace | 11.179 [11.078, 11.753] | 0.076 [0.074, 0.083] | 11.309 [11.201, 11.881] | 10,398,147 / 65,536 / 437,621 |
| six-hour | 21599 | libwebm classic full-load | 9.223 [9.018, 10.961] | 0.027 [0.025, 0.039] | 9.265 [9.059, 11.026] | 9,961,493 / 589,824 / 0 |
| six-hour | 21599 | libavformat | 0.419 [0.400, 0.470] | 198.348 [191.486, 201.067] | 198.989 [192.112, 201.642] | 65,536 / 9,961,472 / 22 |

### Seek resources

| Source | Target s | Adapter | RSS KiB | Allocation requests | Requested bytes | Actual first s |
| --- | --- | --- | --- | --- | --- | --- |
| mono | 0 | unlace | 2564 [2560, 2564] | 35–35 | 72496–72496 | -0.0065000 |
| mono | 0 | libwebm classic full-load | 4356 [4336, 4360] | 141–141 | 174897–174897 | -0.0065000 |
| mono | 0 | Nestegg | 2076 [2076, 2140] | 281–281 | 32104–32104 | -0.0065000 |
| mono | 0 | libavformat | 3268 [3200, 3424] | 476–476 | 122526–122526 | -0.0065000 |
| mono | 0.5 | unlace | 2652 [2556, 2736] | 37–37 | 74040–74040 | -0.0065000 |
| mono | 0.5 | libwebm classic full-load | 4360 [4280, 4368] | 141–141 | 174897–174897 | -0.0065000 |
| mono | 0.5 | Nestegg | 2140 [2076, 2140] | 281–281 | 32104–32104 | -0.0065000 |
| mono | 0.5 | libavformat | 3332 [3204, 3424] | 539–539 | 129452–129452 | 0.4145000 |
| six-hour | 0 | unlace | 6492 [6364, 6528] | 65–65 | 8564516–8564516 | -0.0065000 |
| six-hour | 0 | libwebm classic full-load | 6280 [6276, 6344] | 21731–21731 | 2228765–2228765 | -0.0065000 |
| six-hour | 0 | libavformat | 3268 [3072, 3332] | 438–438 | 110868–110868 | -0.0065000 |
| six-hour | 10800 | unlace | 6404 [6344, 6492] | 64–64 | 8548145–8548145 | 10798.9935000 |
| six-hour | 10800 | libwebm classic full-load | 6280 [6184, 6280] | 21835–21835 | 2250045–2250045 | 10798.9935000 |
| six-hour | 10800 | libavformat | 4356 [4352, 4484] | 3780679–3780679 | 241950817–241950817 | 10799.9135000 |
| six-hour | 3600 | unlace | 6492 [6396, 6532] | 64–64 | 8548144–8548144 | 3598.9935000 |
| six-hour | 3600 | libwebm classic full-load | 6280 [6252, 6320] | 21835–21835 | 2250045–2250045 | 3598.9935000 |
| six-hour | 3600 | libavformat | 4416 [4352, 4420] | 1260679–1260679 | 92550817–92550817 | 3599.9135000 |
| six-hour | 21599 | unlace | 6404 [6392, 6532] | 64–64 | 8548145–8548145 | 21597.9935000 |
| six-hour | 21599 | libwebm classic full-load | 6280 [6280, 6324] | 21831–21831 | 2249877–2249877 | 21597.9935000 |
| six-hour | 21599 | libavformat | 4388 [4288, 4484] | 7560324–7560324 | 466029750–466029750 | 21598.9135000 |

The 64 KiB cache is unfavorable for sparse preparation: unlace's backing reads
cover almost the entire small six-hour file, even though its logical provider
requests in the unbuffered qualification were much smaller. Indexed seek can be
cheap after preparation while preparation itself costs reads and memory. These
are local file read counts, not network savings. FFmpeg's late-window scan and
classic libwebm's retained directory have different tradeoffs on this layout;
the tables show their absolute costs without assuming equivalent index validation.

## Finite truncation observations

An original 220-byte finite Segment fixture is cut at every offset, including
empty and complete input: 221 prefixes for each adapter. Each process is limited
to 256 MiB virtual address space, two seconds CPU, three seconds wall time and no
core dump. All 1,547 processes return without signal or timeout. This bounds only
these executions and is not a memory-safety or denial-of-service proof.

| Adapter | Native zero-exit prefixes | Native End with track | Short prefixes with track + native End | Signals | Timeouts |
| --- | --- | --- | --- | --- | --- |
| unlace | 1 | 1 | 0 | 0 | 0 |
| matroska-demuxer | 22 | 22 | 21 | 0 | 0 |
| Symphonia | 1 | 1 | 0 | 0 | 0 |
| libwebm classic full-load | 30 | 30 | 29 | 0 | 0 |
| Nestegg | 29 | 29 | 28 | 0 | 0 |
| libavformat | 30 | 30 | 29 | 0 | 0 |
| libwebm incremental | 3 | 1 | 0 | 0 | 0 |

Unlace and Symphonia return success only for the complete selected-audio fixture.
Incremental libwebm's native document parser also completes empty input and an
EBML-only prefix; those have no selected track and are not completed audio jobs.
Other native EOF conventions return End on some shortened prefixes after tracks
or packets. Those outcomes are recorded as native API behavior, not proof that
a finite source is complete. General parser correctness cannot be inferred from
this single small source. Every-byte unlace regressions and optional fuzz scope
remain separately documented in [robustness.md](robustness.md).

## Reproduction and retained evidence

Retained local files under `runs/comparison` include immutable source/license
copies, external Cargo.lock, adapter sources, binary/source SHA-256 inventories,
source manifests, full manifests, correctness matrices, every trial, allocation
probe checks and truncation outcomes. Initial adapter experiments are retained
separately; they are not mixed into the final tables. The source rules keep these
raw benchmark artifacts local. This report is a sanitized method and aggregate,
not a portable benchmark distribution or a public release.

From the retained local checkout, the orchestration sequence is:

```sh
bash runs/comparison/build_final.sh
python3 runs/comparison/generate_cases.py
python3 runs/comparison/run_manifests.py
python3 runs/comparison/run_incremental_reuse.py
python3 runs/comparison/run_generated.py
python3 runs/comparison/run_classic_raw.py
python3 runs/comparison/compare_manifests.py
python3 runs/comparison/qualify_benches.py
python3 runs/comparison/qualify_seeks.py
python3 runs/comparison/pin_environment.py
python3 runs/comparison/measure.py
python3 runs/comparison/pin_seek_environment.py
python3 runs/comparison/measure_seeks.py
python3 runs/comparison/check_truncations.py
```

Run the final adapters from the retained artifact inventory. Diagnostic adapter
generators from earlier experiments are not authoritative rebuild scripts.
Build commands use the pinned upstream sources and compiler settings above;
the Rust workspace uses `cargo +1.89.0 build --offline --locked --release`.
C adapters compile separately with `-std=c11 -O3 -Wall -Wextra -Werror`, then link
the unmodified pinned libraries. C++ adapters and libwebm sources use
`-std=c++11 -O3` with the matching parser include directories. The allocation
probe uses `-std=c11 -O3 -Wall -Wextra -Werror -shared -fPIC`.

Source hashes for the timed corpus are listed below. The short Opus, browser and
six-hour identities also link to their existing independent qualification.

| Source | File bytes | Selected packets | SHA-256 |
| --- | --- | --- | --- |
| mono | 11009 | 52 | `12c21d56900699e4e3c9fcde535dd8003c4f9555930a8e5f1a3dabd1ee8fd630` |
| six-hour | 10399259 | 1080001 | `689a57fa5aa23bf9a7be2882b4d4c70f8d79ed6209aafa5c94cf384014e26c2f` |
| browser-mixed | 22330 | 28 | `362cf539cbeef5b2ed8b297afafd8827a90fca9f4e32ba4a08f8d58a4b45b25e` |
| fixed-padding-25000000 | 227 | 3 | `fd9c660f890acc593318c78bd807ac709d875f8459108197015eac966988d309` |
| none-padding-10000000 | 220 | 1 | `3ea398a794923b2d2798f0d887bce1c352eb0f9bdf05c18b849a0f925ccf27c8` |
| negative-raw-time | 204 | 1 | `195838ee979a4a52ef1c4e562f654c2e0d69d500d7171523ad22385c4a2b76cd` |

All correctness-source identities, including untimed cases, follow. Generated
fixtures are original; producer-source rights and capture details are linked above.

| Correctness source | File bytes | SHA-256 |
| --- | --- | --- |
| mono | 11009 | `12c21d56900699e4e3c9fcde535dd8003c4f9555930a8e5f1a3dabd1ee8fd630` |
| stereo | 20356 | `aea4850fd2710dd52d141a2d47e4ba09f45d3c71203065b213af51627417ba3f` |
| browser-audio | 20632 | `a299ed51dd7ade5ff37469612f2f8d74f256d075076ee85de3ccfe96cb8a42fc` |
| browser-mixed | 22330 | `362cf539cbeef5b2ed8b297afafd8827a90fca9f4e32ba4a08f8d58a4b45b25e` |
| youtube | 9730538 | `0a2e331c1743da8feaca135a2cd5ac967d8d1373d6009318630c68649d750955` |
| synthetic-opus | 11009 | `d0a6ac2f284a465fcf4756baf312f84501b5fb3495ec44503ca19cd273e1f362` |
| synthetic-mixed | 7561 | `4a6d0991e7ca554a9b822495c20da9168be8c53f4069a1d1b7d55c6f1d6c84ba` |
| synthetic-vorbis | 4788 | `744f5f016600a914c215db13a5e4fd37816d4839d28d0f5b3ca0ad4d200878de` |
| six-hour | 10399259 | `689a57fa5aa23bf9a7be2882b4d4c70f8d79ed6209aafa5c94cf384014e26c2f` |
| none-padding-0 | 220 | `45e8e176212d34e22e5f8d044fb7dae90841f8aab0734d05cb4e3265ac4a45dc` |
| none-padding-10000000 | 220 | `3ea398a794923b2d2798f0d887bce1c352eb0f9bdf05c18b849a0f925ccf27c8` |
| none-padding--10000000 | 220 | `827373aa1b851bb8b4ec4654c910cd185a0981ef11f38979d9e1d5e909e4d52d` |
| fixed-padding-0 | 227 | `12695ca3b85a2967bc42d8a92920ced615f835299489ecd490da91629a1a52b6` |
| fixed-padding-25000000 | 227 | `fd9c660f890acc593318c78bd807ac709d875f8459108197015eac966988d309` |
| fixed-padding--25000000 | 227 | `56a2017bf0122dd8b7456bea51b668f189ff9f3afc29cb73eb96e2e58e5a06c1` |
| xiph-padding-0 | 229 | `ff4fa6070471bd266417967cb74c3c2e529cc741e39d89559dd405c655b3195d` |
| xiph-padding-25000000 | 229 | `ccae35616a04e48e4197e034e460881af118bb4c99314b02dde0ad59f9665255` |
| xiph-padding--25000000 | 229 | `4cf4425c0a4f3ce9ea37bf5728b9a72cf063bb2474b4950ad5dd1fdba23eec6d` |
| ebml-padding-0 | 229 | `176b2008279b6e586e42d27b917bea42d90baac3f2d1e137e0be5fdfe813cc77` |
| ebml-padding-25000000 | 229 | `130779f18c56d5e1d5e292c816fb2fc5571a4b48e3fa1e11082ee97ca3ea0e05` |
| ebml-padding--25000000 | 229 | `0aa43c06e87bee90e5f49a0716535b1fc6f3adaf2754cbd42c6451290c7fe736` |
| negative-raw-time | 204 | `195838ee979a4a52ef1c4e562f654c2e0d69d500d7171523ad22385c4a2b76cd` |
| minimum-relative-time | 204 | `f1f35acf1c50b9d76a296ccda92b63884f252ebfabf6d3e8156ee8c8b258aab6` |
| two-audio-disabled-first | 293 | `8fd88109cd0ce147142af33def1836fc0ca33520f6551bc011daa4e1f222618e` |
| preroll-one-nanosecond | 204 | `d627c88455991680984bcc87fdd3f8b2792c26fc906cd9e84090a915729ee076` |

Final executable SHA-256 values distinguish the measured adapter variants.

| Artifact | SHA-256 |
| --- | --- |
| unlace sequential | `c363941363394c650a7f02efc53694e8d2a1da1360ce207b017f4b86fe8d9ca1` |
| unlace seek | `3c1b4a993302e1b956cdc97d1cfd78d1aa32e5a5889a6637b5c911a0449d3394` |
| libwebm classic full-load sequential | `9f6e22129ae0f92b86b50d0f0e7e8bab1e54560baf8dbe4a282bd2aa759408fe` |
| libwebm incremental sequential | `8ea0f2fd6c13273baa08fc3736d93e3be86310e5b9f5cae4db3a92cf3a332d3a` |
| Nestegg sequential | `768e1bd15df6225cca217fb961f3f4e2c753a2b39b88af20282fa98952f27104` |
| libavformat sequential | `90fd3ea24ce11940a129daaa561629a967935e940b59731a4b9d3baed4effb88` |
| libwebm classic full-load seek | `b9b10fc504c14e348392f68892449b48ff774b8f07a42cbb7e5704fedd06c7ae` |
| Nestegg seek | `7a5c8dde36ea0b5ff588a2c9485320d8e183b9f9a07d07d153addc84d814059f` |
| libavformat seek | `d2b2b063137d879321fa71aa936824dafca100491fd333f6c0bdf3f4fdd947e4` |

These measurements leave concrete limits: a narrow codec/feature subset, a
single machine, small original adversarial inputs, silent long-duration packets,
conditional FFmpeg precision reconstruction, fresh-handle seeks, different native
completion guarantees and no equivalent-work writer comparison yet. Ordinary
unlace checks remain dependency-free and do not require these research tools.

## Later writer observation

The separate [writer qualification](writer-qualification.md) records a laced Opus
fixture for which normal pinned FFmpeg playback repeats whole-block padding on
each frame. Its public packet positions permit correct block grouping in the
independent decode adapter. The measurements in this report compare selected
packet metadata, not playback or decoded trim; they are unchanged by that later
observation. Packet-field admission must not be read as a playback guarantee.
