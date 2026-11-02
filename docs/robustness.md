# Malformed input, continuation and completeness qualification

This record qualifies the current bounded reader subset, based on library source
commit `c9958e0`. It is not a safety proof, full Matroska conformance statement,
codec-decoding result or performance ranking. The executed campaigns and original
fixtures below add evidence to the ordinary structural/timing tests.

A later [writer state qualification](writer-fuzzing.md) uses a separate versioned
input format and records its own campaigns. Historical reader counts below are
not combined with writer coverage.

## Observable state and recovery

| Outcome | Meaning and continuation |
| --- | --- |
| Sequential `NeedInput` | WouldBlock; retain the same owned reader and resume when bytes become available |
| `Yield` | Per-poll work exhausted; resume the same state |
| Framing `SourceEof` | Final EOF at a structural boundary; not a complete-container validation by itself |
| Demux `End(FiniteSegment)` | Required supported metadata and finite Segment/source boundaries observed through EOF |
| Demux `End(UnknownSizeSegment)` | Supported structure ended at EOF; producer-intended completeness remains uncertain |
| Seek suffix End | Only the suffix from the last committed Cluster was read; skipped media and time-interval completeness are not inferred |
| Truncation/corrupt/unsupported | Distinct typed errors; sequential framing/demux state is terminal after fatal errors |
| Source I/O | Preserve original cause/location where provided; a core I/O failure is terminal, while positional index transactions may be retried |
| Resource refusal | Explicit limit error; never successful EOF or a repaired container. Sequential fatal limits poison state; seek/read allowances have their documented retry scope |
| Cancelled seek | No new active packet position committed; prior buffered packets remain usable |

There is no automatic corruption resynchronization or optional recovery mode.
Unlace does not scan payloads for plausible IDs and silently continue after an
invalid boundary. Index fallback is a bounded walk of actual element/block
prefixes, not corruption recovery. Ordinary tests verify that late corruption or
I/O cannot turn earlier packet delivery into successful completion.

There is no exported/imported checkpoint or portable resumable-state format.
Sequential readers own their source, partial headers, buffers and stack; recovering
the source with `into_inner` discards that state. The seek wrapper pins an immutable
provider snapshot, checks it even before publishing buffered packets, and accepts
only attested Cluster boundaries in that snapshot. Observed A-B-A token changes
remain latched. Generic Read + Seek cannot detect same-length external writes;
its caller must keep bytes immutable. Version-aware providers must honor their
revision contract. These are source contracts, not cryptographic authentication.
See [streaming](streaming.md), [demuxing](demuxing.md), [sources](sources.md) and
[seeking](seeking.md) for the full API and limit distinctions.

## Coverage-guided fuzzing

Optional [portable tools](../tools/fuzz/README.md) build Rust test executables
without third-party Cargo dependencies or unsafe code. Ordinary Cargo builds,
tests and docs do not need LLVM tools or network access. All generated corpus
state, profiles, logs and findings stay in ignored `runs/` directories.

Five modes exercise VINT/Block/Opus parsers, nested/unknown-size hierarchy,
demux/timestamp/trim transitions, index/seek transitions and source revision/budget
state. Whole-input and short/interrupted/paused-input runs must produce identical
normalized events or packet fields, offsets and terminal results. Seek requests
have an independent signed i128 control, allowing time mutation without damaging
the document header. Source actions check latched identity changes and explicit
read/work bounds. Successful frames must remain exact bounded views. Poisoned
framing/demux state must not read again. These are metamorphic and state oracles;
they are not an independent implementation of every format rule.

Seeds are original synthetic bytes: finite/unknown masters, all lacing forms,
Opus initialization, large timestamps/scales, finite indexes, track/trim metadata
and source-action sequences. Fuzzed frame-count allowance is 32; ordinary block
tests separately cover all 256 frames. Portable dense-lace seeds fit 32,000 empty
frames in a small valid container. The test harness has explicit poll/trace
bounds large enough for these cases. Invented packet bytes in fuzz seeds are not
a codec qualification.

Mutations change bits/bytes, insert/delete/duplicate data, truncate input and
insert EBML-specific tokens. Feedback retains inputs that reach new positive
compiler source-coverage segments in library files. A segment key is its
file/line/column; this is not AFL/libFuzzer edge feedback or branch coverage.
Coverage is obtained using Rust `-C instrument-coverage`, with `opt-level=1`,
debug info and overflow checks enabled. No sanitizer or Miri run is implied.
Each target process is limited to 8193 input bytes, 512 MiB address space, two CPU
seconds and three wall seconds. Parent/LLVM-tool processes are outside those child
limits. Large/adversarial inputs beyond this fuzz scope require other tests.

The two initial campaigns ran on Linux 7.0.0-38-generic x86_64 with Rust 1.89.0
(`29483883eed69d5fb4db01964cdf2af4d86e9cb2`, LLVM 20.1.7). Compatible local LLVM
coverage tools reported `20.1.5-rust-1.88.0-stable`; profile merging/export succeeded.
Python 3.14.4 drove the seeded mutations and line-by-line checks.
They used the original prototype harnesses retained locally. Later campaigns
validated the portable five-mode tool and denser valid output cases.

| Phase | RNG seed | Mutations | Seed replays | Executions | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| Initial framing/demux/index campaign | 6012027 | 20,000 | 84 | 20,084 | No failure |
| Independent seek-time/source-state campaign | 6012028 | 15,000 | 36 | 15,036 | No failure |
| Portable runner with retained corpus | 6012029 | 2,000 | 159 | 2,159 | No failure |
| Dense-lace portable campaign | 6012031 | 1,000 | 161 | 1,161 | No failure |
| Final test-only portable build | 6012032 | 1,000 | 163 | 1,163 | No failure |

These total 39,000 mutations and 39,603 executions including seed replays, plus a
separate 100-mutation pilot and explicit replay checks. No new library crash,
panic, timeout or oracle mismatch was found. Thus there was no new crashing fuzz
input to minimize. Coverage-increasing inputs are retained locally. Existing
portable regressions for previously reproduced parser/seek defects remain in the
normal suite; no successful fuzz run replaces them. The bounded reducer was
self-checked against a synthetic failure predicate, not reported as a library bug.

Replaying the retained inputs from the first two campaigns reached 3,284 of 4,004
LLVM library source regions and 2,304 of 2,718 executable source lines. These are
coverage of those instrumented fuzz targets, excluding the ordinary test suite,
not a project-wide test coverage score. Representative region counts are:

| Source | Covered / mapped regions |
| --- | ---: |
| Integer parsing | 78 / 95 |
| Block/lacing | 242 / 288 |
| Streaming hierarchy | 438 / 516 |
| Integrated demux | 983 / 1,224 |
| Timing relations | 65 / 93 |
| Opus initialization | 122 / 151 |
| Index parser/destination walk | 971 / 1,080 |
| Seek wrapper | 234 / 325 |
| Positional source | 147 / 221 |

The remaining four covered schema regions out of eleven complete the total.
Unexecuted regions include API/formatting functions and operational error paths;
not all branches or allocation failures were exercised. Rust's exported branch
and MCDC denominators were zero, so no branch coverage percentage is claimed.
The final portable campaign separately reached 3,251/4,004 regions; corpus and
harness scope differ, so this is not a regression or a directly interchangeable
score. It was replayed against its recorded source hashes by `report.py`.

Example reproduction from a fresh private checkout:

```sh
python3 tools/fuzz/run.py --out runs/fuzz-new \
  --llvm-tools /path/to/compatible/llvm/bin --cases 20000 --seed 6012028
python3 tools/fuzz/report.py runs/fuzz-new \
  --llvm-tools /path/to/compatible/llvm/bin
```

Use the exact retained corpus/tool version to reproduce historical execution
counts; an evolving corpus or target changes the mutation sequence. Each new
campaign records source/tool hashes, ordered starting-input hashes, compiler/tool
versions, limits and RNG seed. The tool refuses existing output directories and
keeps all artifacts local. A finding must be reproduced, reduced and promoted to
an ordinary regression before changing tested production behavior. Bounded
reduction is heuristic, not proof of a globally minimal input.

## Every-byte source and truncation evidence

The ordinary tests already cover every-byte finite-fixture truncations and
sequential header/payload I/O failures, plus source pauses/interruptions at every
position. Three additional seek regressions extend this record:

- Every possible preparation-byte failure preserves the absolute source error.
  Metadata-core I/O failure remains terminal without another read. Positional
  index-read failure can resume its transaction after clearing the source fault.
  Bytes skipped by preparation do not falsely count as inspected.
- Every suffix-byte failure remains a located I/O error, including ancillary index
  payloads; earlier packets do not turn it into End. Further polls remain terminal
  and perform no read.
- A bounded before-first-cue scan refuses work exactly at its call allowance and
  leaves the previous active suffix intact. A five-Cluster/50-call fixture is the
  smallest Cluster count for this shape; four Clusters do not trigger refusal.

These are original synthetic regressions, not imported media or claimed fuzz
crashes. Other source tests exercise wrong read counts, premature provider EOF,
missing ranges, same-length revision replacement, repeated backward seeks,
cancellation and source checks when packets are already buffered.

## Six-hour container qualification

An original fixture generator repeats an original FFmpeg/libopus silence packet,
not someone else's fixture or recording. Producer: pinned FFmpeg 7.0.2-static;
Opus mono 48 kHz, 32 kbit/s VBR request, 20 ms packet duration. The encoded silence
packet is `f8fffe`; the generator independently writes finite EBML/Segment,
Info/Tracks, 21,601 one-second Clusters, Cues and SeekHead. The final packet uses
BlockGroup DiscardPadding. The complete file is 10,399,259 bytes, SHA-256
`689a57fa5aa23bf9a7be2882b4d4c70f8d79ed6209aafa5c94cf384014e26c2f`.

All 1,080,001 selected packet payloads and CodecPrivate match pinned Nestegg
commit `767aab25013acefbdcc6d68a2a7a2c9081303e3f` and libavformat 61.1.100.
The ordered length-prefixed packet digest is
`6c0fa3b76ead4402c47bf277260d818c9c2db3630a94a0dc4df23d362664a561`.
Every raw timestamp matches `packet_index * 20,000,000 ns`; each presentation time
subtracts CodecDelay 6,500,000 ns once. Pre-skip is 312 samples and final padding
648 samples (13,500,000 ns), independently reported by libavformat's final packet
side data. The final presentation end is exactly 21,600,000,000,000 ns (six hours),
followed by finite Segment and final source EOF evidence. Manifest processing is
line-by-line; neither the complete media nor complete packet result is buffered.

This is container/sample arithmetic and packet fidelity, not decoding six hours
of PCM. Repetition limits content diversity; timestamps, count and final EOF were
checked in addition to identical packet bytes. The separate short mono/stereo
[decoded-span qualification](opus-timing.md) is not replaced by this trial.

The default 65,536-call seek allowance explicitly refuses the before-first-cue
fallback for this file. It needs 151,214 reads to scan all first selected blocks;
the error remains `Source(Limit)`, not completion or silent relaxation. This
unfavorable result is retained. With an explicit 500,000-call allowance, requests
at 0, 10,800, 3,600 and 21,599 seconds match full sequential suffixes and requested
one-second intervals. Opening reads 566,483 provider bytes across preparation
calls; the indexed three-hour seek reads 305 bytes. The zero-time fallback reads
242,374 bytes. These are provider reads, not latency, disk/network traffic,
allocation or superiority measurements. Source checks perform additional seeks.

Raw generator/command evidence and comparisons remain under ignored
`runs/completeness-qualification/`, `runs/demux-qualification/` and
`runs/fuzz-qualification/`. Ordinary tests need no external codec/LLVM executable,
media download or large fixture. Further platform coverage, more realistic
multi-hour audio, allocation-failure injection, longer/more targeted fuzzing and
matched competitor resource/performance trials remain broader qualification work.
