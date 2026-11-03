# Original seek qualification

This records the original selected-audio seek campaign before the later source
performance milestones. Counts and read costs describe that campaign, not the
current suite or a latency guarantee. See the [current seek contract](../seeking.md)
and [evidence index](../README.md#qualification-and-audits) for subsequent work.

Eighteen original synthetic seek tests check exact forward/backward packet
suffixes, selected-track identity, raw/effective timing, fixed laces and final
padding, missing/video-only cues, group/no-relative hints, every track VINT width,
forged pointers into matching payload bytes, incorrect times/tracks/relative
positions, SeekHead bounds/overflow, finite truncations at every byte, every-byte
range holes, short/interrupted reads, preparation and packet cancellation, source
replacement, unsupported CRC classification and explicit resource/work limits.
Four defects were reproduced before correction: eight-byte VINT mask overflow,
request-overflow poisoning, treating unsupported index CRC as corrupt input, and
misclassifying a missing Cluster timestamp as no available access point.

Local corpus qualification uses the eight original/pinned sources in
[packet qualification](../packet-qualification.md) and [Opus timing](../opus-timing.md).
The local adapter compares every suffix packet byte and all packet fields with
full sequential iteration on the identical file and selected track. It uses a
fresh sequential file reader per query and repeats forward/backward requests on
one seek reader. It also compares each requested one-second timestamp interval against full
iteration (including packets before the skipped boundary) and requires both
paths to reach their final completion. All corpus packet timestamps in this
check are determinate.
Baseline ordered packet bytes on these producer cases were independently checked
as documented in those reports; this seek check is same-library suffix
comparison, not a new independent decoder or competitor seek comparison.

Four queries per source passed (32 total). The seven short sources used
0, 300, 100 and 2000 milliseconds, in that order. Big Buck Bunny used
0, 300, 20 and 590 seconds. The second query's provider reads are below; opening
is shown separately and is paid once per reader. Repeated reads count repeatedly.

| Source | Open bytes | Seek bytes | Matched suffix packets | Full selected packets | Second-query path |
| --- | ---: | ---: | ---: | ---: | --- |
| Browser Opus | 488 | 108 | 25 | 28 | Missing Cues scan |
| Browser VP9/Opus | 589 | 120 | 26 | 28 | Missing Cues scan |
| Synthetic Opus | 483 | 373 | 52 | 52 | Selected cue |
| Synthetic Vorbis live | 3722 | 9 | 21 | 21 | Missing Cues scan |
| Synthetic VP9/Opus | 552 | 10 | 21 | 21 | No selected-track cue scan |
| Opus mono trim source | 483 | 373 | 52 | 52 | Selected cue |
| Opus stereo trim source | 484 | 373 | 52 | 52 | Selected cue |
| YouTube Big Buck Bunny | 1781 | 3505 | 15325 | 29825 | Selected cue |

The 300-second Big Buck Bunny query starts at Cluster byte 4,785,185 and actual
selected time 289,994,500,000 ns, with declared preroll 80,000,000 ns. Reading its
suffix used 4,945,353 provider bytes; full iteration used 9,730,538. These are
actual direct `File` read counts in a local correctness adapter, not disk/network
traffic, cold/warm cache measurements, latency, allocations or throughput. Source
checks perform additional seeks. No numerical performance ranking is claimed.

Checks ran on Linux 7.0.0-38-generic x86_64, Rust 1.89.0
(`29483883eed69d5fb4db01964cdf2af4d86e9cb2`, LLVM 20.1.7), debug builds with
standard limits, no external runtime/dependency in ordinary tests. Source hashes,
rights and producer versions are in the linked reports. Commands used the local
`runs/seek-qualification/check` adapter, for example:

```sh
runs/seek-qualification/check runs/demux-qualification/youtube-bbb.webm \
  0 300000000000 20000000000 590000000000
```

Raw adapters, sources and JSONL evidence remain local under ignored `runs/`.
Sustained fuzzing, multi-hour hostile/index coverage, additional platforms,
matched competitor seeks and performance qualification remain separate work.
