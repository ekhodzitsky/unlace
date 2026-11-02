# Optional bounded fuzzing

These development tools compile test targets directly with Rust 1.89.0 and its
LLVM source-coverage instrumentation. They add no Cargo dependency, build script,
networking or requirement to ordinary builds/tests. The Python runner currently
qualifies Linux process limits only and needs compatible `llvm-cov` and
`llvm-profdata` binaries supplied explicitly. Rust's optional `llvm-tools-preview`
component is one way to obtain them; no tool installation is automatic. See the
[official instrumentation documentation](https://doc.rust-lang.org/rustc/instrument-coverage.html).
Keep build sources and campaign artifacts unchanged while a campaign is active.

Run from the repository, using a fresh output directory below ignored `runs/`:

```sh
python3 tools/fuzz/run.py --out runs/fuzz-example \
  --llvm-tools /path/to/compatible/llvm/bin --cases 20000 --seed 6012028
python3 tools/fuzz/report.py runs/fuzz-example \
  --llvm-tools /path/to/compatible/llvm/bin
```

`run.py --help` lists options. `--target reader` is the default and retains the historical five-mode
`reader-v1` format. `--target writer` selects the independent `writer-v1` format
described below. `--replay DIR` adds inputs in the selected format. Campaign
replay/minimization rejects a conflicting target in an adjacent manifest; raw
input directories without a manifest remain the caller's responsibility. Additional inputs are bounded to 8193 bytes and copied to the
new corpus; their ordered hashes are recorded. Existing campaign directories
are refused. Source/tool hashes, compiler/LLVM versions, limits and the RNG seed
are recorded before compilation. Library source changes are refused by coverage reporting.
Compilation errors, target failures and tool failures return nonzero; no branch
or safety claim is inferred from a successful run.

For `reader-v1`, the first byte selects a mode modulo five:

| Mode | Input and oracle |
| --- | --- |
| 0 | VINT, Block/SimpleBlock and OpusHead parsing; frame bounds, exact repeated frame views |
| 1 | Synthetic hierarchy; equal normalized events, offsets and terminal results over whole vs short/interrupted/paused input |
| 2 | Audio demux; equal tracks, packet bytes/fields, offsets and final results over both input schedules |
| 3 | Signed little-endian i128 request followed by a container; prepare, seek forward/backward, compare actual results and full suffixes over positional sources |
| 4 | Pairs of source-control bytes; revision changes, repeated checks/budget resets, short reads, WouldBlock/Interrupted/I/O and replacement during a read |

Mode 3 uses its first 16 payload bytes as an independent request, so changing a
seek time need not damage the container header. Sources preserve immutable tokens
within their contract, and observed source changes must stay latched. Valid Opus
initialization seeds use invented packet bytes; no decoding is claimed. Dense
empty-lace seeds exercise large legal packet counts within small input bounds.
Assertions and unwraps belong to a `cfg(test)` fuzz executable, compiled with
`--cfg test`; production library assertions/recovery behavior are not added.

Each input process has 512 MiB address-space, two CPU seconds and three wall
seconds of allowance. Target metadata/payload/track/frame/index/header/poll/trace
bounds are explicit in `target.rs`. Raising these target-specific bounds changes
the qualified scope. The parent and LLVM reporting processes are not covered by
the child address-space limit. Feedback retains inputs reaching new positive
source coverage segments in `src/`, identified by file/line/column. This is not
branch coverage or libFuzzer/AFL edge instrumentation. Mutation uses deterministic
bit/byte changes, insertion/deletion/duplication, truncation and EBML tokens.

A failure stores input and stderr under `findings/` and stops the campaign. Reduce
it in a separate fresh directory before changing tested behavior:

```sh
python3 tools/fuzz/run.py --out runs/fuzz-minimize \
  --llvm-tools /path/to/compatible/llvm/bin \
  --minimize runs/fuzz-example/findings/HASH.bin
```

Reduction preserves the process status and panic location when available, with
a cooperative 30-second/1000-trial budget; a final trial may run up to its
three-second wall allowance beyond that budget. Add `--target writer` when
reducing a writer finding. It is a bounded deletion heuristic,
not a guarantee of global minimality. Classify harness/resource failures separately
from library defects. Promote original portable counterexamples into normal tests
before a production fix; keep raw campaigns and profiles local. A resource limit
is not corrupt input, EOF or successful recovery.

The report replays unique retained inputs, merges their profiles and exports LLVM
source region/line/function coverage for library files. It is separate from
ordinary test-suite coverage and does not include a branch/MCDC denominator.
Reports and compiler data stay in the campaign directory. There is no automatic
corruption resynchronization or checkpoint import. See
[the qualification record](../../docs/robustness.md) for exact executed scope,
results, uncovered areas, corpus identity and limits.


## Writer state target

```sh
python3 tools/fuzz/run.py --target writer --out runs/writer-fuzz-example \
  --llvm-tools /path/to/compatible/llvm/bin --cases 20000 --seed 6110211
python3 tools/fuzz/report.py runs/writer-fuzz-example \
  --llvm-tools /path/to/compatible/llvm/bin
```

Every input runs live and indexed writers with clean and injected-fault sinks.
The target compares successful output against caller records, including exact
bytes, timestamps, lacing, flags, signed whole-block/once-only padding and track
metadata. Receipt counts, end offset and finite/live completion must agree.
Preflight errors that leave Writing must preserve position, buffered size and
sink call count. Failed/Finished writers must refuse subsequent write/flush/finish
without I/O. Each operation must respect its I/O allowance. This tests source
regions, not all paths or exhaustive state combinations.

`writer-v1` begins with 20 control bytes (missing bytes read as zero):

| Byte | Meaning |
| --- | --- |
| 0 | Low two bits select scale 1/1000/1000000/u64::MAX; bits 2/3 remove default duration/disallow lacing; bits 4/5 add/disable a second track |
| 1, 2 | Fault mode modulo 14; operation occurrence 1 + byte modulo 64 |
| 3 | Zero means full writes; otherwise maximum chunk 1 + byte modulo 32 |
| 4 | Cluster span 0/20 ms/1 s/u64::MAX, low two bits |
| 5 | Pending Cluster cap 0/17/128/1024 bytes, low two bits |
| 6 | Metadata cap 0/64/512/4096 bytes, low two bits |
| 7 | Cue cap 0/1/4/32, low two bits |
| 8 | Low two bits select lace header cap 0/8/128/1024; next two select frame cap 0/16/256/1024 |
| 9 | Low two bits select I/O calls 1/16/128/1024; bit 2 restricts tracks to one |
| 10 | Low bit selects cue interval 0/1 s |
| 11 | Writing application normal/empty/NUL, modulo three |
| 12–15 | Bits in byte 12 mutate Opus version, mapping family or delay using bytes 13–15 |
| 16 | Low bit reinterprets block ticks as unsigned u64 |
| 17 | Second track codec modulo seven: Opus, signature-only Vorbis, malformed Vorbis, VP8, VP9, unknown audio, unknown video |
| 18 | Video private-data and zero-width variations |
| 19 | Reserved |

At most 32 actions follow. Tag low bits select block/flush/finish/abort. Abort
omits automatic finish and drops buffered work without claiming success. Block
records contain track selector (modulo three plus one), eight signed little-endian
tick bytes, flags, frame count modulo 11, lace selector (none/fixed/Xiph/EBML),
signed i8 padding in milliseconds, duration in scale ticks, reference control and
payload length in four-byte units, followed by up to 1020 payload bytes. Missing
fields read as zero; payload reads stop at EOF. Timestamp is ticks times scale
minus configured delay. Tag bit 7 requests i128::MAX or MIN (flags bit 4).
Flags bits 0/1/2 mean non-key/invisible/discardable, bit 3 selects laced frames,
bit 5 requests zero duration and bit 6 requests unequal fixed frames. References
use the control's high two bits as count and low six bits minus 32 as the first
offset; remaining offsets are -20 and 20. Fixed laces normally repeat the payload;
other frames use bounded deterministic prefixes. Duration zero means absent.

Fault modes 0–13 select none, write error, write zero, flush error, seek error,
one interrupted write, WouldBlock, invalid write count, wrong seek result,
normal I/O, repeated interruptions, interrupted flush, interrupted seek and
another one-interrupted-write control. Occurrences count calls of the relevant
operation; repeated interruptions affect every operation until its budget ends.
Mode 9 deliberately supplies a fault-free control.

Input is at most 8193 bytes, output 128 KiB per worker, Block 4096 bytes, eight
frames, two tracks and index 2048 bytes. At most 256 accepted frame records are
retained. Clean and fault workers execute sequentially. Successful enabled audio
tracks are demuxed independently to End; video payloads are receipt-counted but
not decoded or independently inspected by this target. Encoded bytes and Vorbis
signatures are invented fixtures, not playable codec qualification. Constructor
failure consumes its sink and cannot be inspected through a returned writer.
Allocator-failure injection, huge outputs, durability and external publication
are outside this target.

For manual independent inspection only, compile the test target and pass
`--emit EXISTING_DIRECTORY`. Successful receipts create fixed live/indexed
clean/fault names using create_new. The directory must resolve to a child of
ignored project `runs/`; existing files are refused. Campaigns never pass this
flag or export every mutation. See [writer qualification](../../docs/writer-fuzzing.md)
for executed campaigns, control mutations and coverage gaps.
