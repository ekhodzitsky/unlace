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

`run.py --help` lists options. `--replay DIR` adds input files in the current
five-mode format. Additional inputs are bounded to 8193 bytes and copied to the
new corpus; their ordered hashes are recorded. Existing campaign directories
are refused. Source/tool hashes, compiler/LLVM versions, limits and the RNG seed
are recorded before compilation. Library source changes are refused by coverage reporting.
Compilation errors, target failures and tool failures return nonzero; no branch
or safety claim is inferred from a successful run.

The first byte selects a mode modulo five:

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
three-second wall allowance beyond that budget. It is a bounded deletion heuristic,
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
