# Reader buffer reuse in private 0.2.1

This compatible refactoring follows the [0.2.0 measurements](performance-0.2.md),
which showed slower long-file iteration despite low allocation counts. It changes
private buffer placement, not parsing, timestamp/trim rules, work/byte limits,
packet ownership, source identity or completion semantics. No public API migration
is required. Timing comparisons are separate from the cost profiles below.

## Measured cost and selected change

Hardware perf events are unavailable under the host's perf_event_paranoid=4.
No system policy was changed. Locally extracted Valgrind 3.26.0 Callgrind and
matching glibc debug information supplied instruction, call and memory-reference
profiles instead. These tools remain optional local research state, without
Cargo dependencies or normal build requirements.

Complete 120-second silence and natural-speech fixtures each expose 6,000 packets.
Both profiles showed repeated memset calls from Demuxer::prepare_packets and
Demuxer::poll: a maximum 256-entry frame-range table was initialized/moved for each
selected block, and advance initialized an 8 KiB scratch array repeatedly. A
full six-hour instrumented attempt was stopped after the complete small profiles
identified this cost; it is not counted as completed input or a timing result.

The range table now belongs to Demuxer and only validated active entries are
replaced. The small pending-packet metadata carries the active count/index.
The initialized scratch array also belongs to Demuxer and only the current
successful read prefix is appended to metadata/block storage. Restart clears
pending state; a fresh selected block fills its complete active range prefix.
No new packet can replace that table while an earlier packet remains pending.

| Complete 6,000-packet profile | Before instructions | After instructions | Before write references | After write references |
| --- | ---: | ---: | ---: | ---: |
| Silence | 97,461,234 | 20,887,145 | 78,720,983 | 4,179,127 |
| Natural speech | 99,193,768 | 22,619,677 | 78,749,768 | 4,207,912 |

Callgrind's model expands string operations and simulates caches. These are
instrumented counts, not hardware cycles, sampled CPU percentages or real-time
speedup factors. The same strict field consumer and 64 KiB source buffering run
before/after; packet count, payload count, digest and backing reads agree.
Immutable source/tool/binary/fixture identities and raw profiles remain local.

## Memory and verification

The scratch has moved from a transient advance stack array to retained instance
storage. On this x86_64 host, Demuxer<&[u8]> changes from 11,136 to 19,328 bytes,
and SeekReader<SeekSource<Cursor<&[u8]>>> from 11,616 to 19,808 bytes, alignment 16.
That is exactly 8,192 extra retained bytes per instance. There is no new heap
allocation or recording-sized buffer; each owned reader has its own fixed scratch.
Multiple simultaneous readers must include this cost in application budgets.
Peak process RSS and type size describe different things.

All mandatory Rust 1.89.0 gates pass: formatting, denied-warning all-target
Clippy, offline locked debug/release tests, warning-free rustdoc and the dependency
checker. The suite has 144 integration tests, two unit tests and five doctests.
A focused regression crosses a 256-frame lace, an unselected payload larger than
scratch, empty/single/two-frame blocks and paused/interrupted short reads. It passes
both the old and new implementation, protecting unchanged observable behavior.

The complete 25-input historical manifest replay passes. Fourteen live/indexed
remux outputs retain exactly the earlier SHA-256 values. All ten natural/large
native selected-track jobs again match 9,698,068 packets through finite End.
Three full 17-byte/WouldBlock manifest comparisons, three latched resource
refusals and eight forward/backward seek suffix jobs pass against the new reader.
The original [full decoded comparisons](natural-corpus.md) are not repeated:
all encoded bytes, initialization and trim metadata supplied to that decoder are
unchanged. No new decoding guarantee is inferred beyond those inputs.

Fresh reader and writer campaigns run 3,000 and 2,000 mutations with 158 and 179
seed replays respectively, without a finding. Final source profiles and retained
corpus reports are pinned separately from historical writer/reader campaigns.
The clean plus seven intentional writer-oracle controls all behave as intended.
The [writer record](writer-fuzzing.md) explains their limits and prior corrections.

A separately authorized second agent reviewed this refactoring, including stale
ranges/scratch, short/paused reads, restart/cancellation and source states. Its
targeted mixed-length regression and all 21 seek tests pass. No concrete production
defect was found. This second-agent review is not external security certification
or proof that every path has been exercised.

## Real-clock comparison

Equivalent-work qualification and repeated measurements are recorded separately
when complete. The cost-model reductions above must not be advertised as a
throughput ranking. Public distribution and broader production decisions still
follow the [readiness assessment](release-readiness.md).
