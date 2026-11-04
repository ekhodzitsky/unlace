# Documentation

Current source: **0.2.4**, public repository, experimental crate published on crates.io. The repository README is the
quick start; this index is the map for contracts, checks and retained evidence.
Dated reports describe their pinned source and campaign. Their historical test
counts, failures and unfinished-work statements are not current task status.

## Current contracts

| Document | Purpose |
| --- | --- |
| [Design](design.md) | Product scope, invariants and consumer responsibilities |
| [Incremental framing](streaming.md) | Schemas, unknown sizes, payload chunks and continuation |
| [Block parsing](blocks.md) | Borrowed frames and all four lacing modes |
| [Audio demuxing](demuxing.md) | Track selection, packet lifetimes, supported subset and completion |
| [Opus timing](opus-timing.md) | Initialization, delay, preroll, timestamps and whole-block padding |
| [Positional sources](sources.md) | Immutable revisions, missing ranges and read budgets |
| [Seeking](seeking.md) | Attested destinations, fallback, prepared ownership and exclusive sessions |
| [Writing](writing.md) | Encoded blocks, live/indexed output, failures and checked finalization |

Examples live in [examples/](../examples). They are correctness and ownership
examples, not codec implementations or benchmarks. Generate API documentation
with `cargo doc --offline --locked --no-deps`.

## Development and release policy

| Document | Purpose |
| --- | --- |
| [Verification](verification.md) | Required local commands, example smoke checks and evidence boundaries |
| [Qualification protocol](qualification.md) | Equivalent-work admission before performance claims |
| [Version policy](versioning.md) and [changelog](../CHANGELOG.md) | SemVer, migration and source milestone history |
| [Experimental scope](experimental-scope.md) | Compatibility, private reporting and supported distribution scope |
| [Release and production assessment](release-readiness.md) | Current gates, scoped local go and remaining application/publication prerequisites |
| [Optional fuzz tools](../tools/fuzz/README.md) | Separate bounded reader/writer campaigns and oracle controls |

Use Rust 1.89.0. Ordinary builds and tests are offline and dependency-free;
optional qualification tools and media are not ordinary test dependencies.
[Repository instructions](../AGENTS.md) define the local Backlog workflow.
[Local-only rules](../agents/local-only-files.md) keep raw research, operational
media, logs, credentials and machine state out of tracked source and packages.

## Qualification and audits

| Report | Evidence scope |
| --- | --- |
| [Selected packets](packet-qualification.md) | Synthetic, MediaRecorder and completed YouTube packet comparisons |
| [Opus timing evidence](opus-timing.md#decoded-sample-qualification) | Scoped independent mono/stereo decoded trimming |
| [Speech](speech-qualification.md) | Own multilingual packet and PCM checks |
| [Natural and large corpus](natural-corpus.md) | Long speech, mixed tracks, packet/decoded spans and provenance |
| [Robustness](robustness.md) | Malformed input, continuation, truncation and bounded reader fuzzing |
| [Writer output](writer-qualification.md) | Native readback, packing, long/mixed output and retained playback disagreement |
| [Writer fuzzing](writer-fuzzing.md) | Campaign-specific state/failure oracles and their corrections |
| [Original objective audit](objective-audit.md) | Historical delivered requirements before the 0.2.0 source audit |
| [0.2.0 source audit](source-audit.md) | Confirmed fixes, module coverage and remaining scope |
| [Second-agent review](independent-review.md) | Scoped same-session findings, controls and evidence review |
| [Original seek campaign](history/seek-qualification.md) | 32 original suffix/window checks and read counts |
| [Early verification milestones](history/verification-milestones.md) | Preserved verification narrative through 0.2.1; historical counts and status |

## Performance and competitors

| Report | Evidence scope |
| --- | --- |
| [Competitor selection](comparison.md) and [implementation studies](implementation-studies.md) | Dated primary-source/API/test research; no current-market ranking |
| [Initial matched report](matched-comparison.md) | Pinned selected-audio equivalence gates, time, RSS, allocations and seeks |
| [0.2.0 performance](performance-0.2.md) | Post-audit refresh and common-cache control |
| [0.2.1 reader refactoring](reader-performance.md) and [paired performance](performance-0.2.1.md) | Buffer reuse, retained memory and equivalent-work comparisons |
| [0.2.2 bounded seek reads](seek-performance-0.2.2.md) | Grouped reads, refusals and fixed-budget follow-up |
| [0.2.3 fallback reuse](seek-cache-0.2.3.md) | Original-work charging, storage cost, first-use and repeat timings |
| [0.2.4 prepared sessions](prepared-sessions-0.2.4.md) | Original-provider lifetime, preparation, windows and fresh-reader comparison |

## Consumer pilots

| Report | Disposition |
| --- | --- |
| [Original reference pilot](reference-pilot.md) | Retains large ranged refusals/deadline misses and staged-output limits |
| [GigaSTT integration](gigastt-integration.md) | Actual consumer source/codec/model pilot; no-go for transparent replacement, with explicit admission, trim, PCM and failure differences |
| [0.2.4 bounded concurrent pilot](consumer-pilot-0.2.4.md) | Scoped go for a prepared local hash consumer with explicit Busy handling; no deployed application approval |

The [accepted product decision](decisions/0001-product-direction.md) preserves
the founding rationale. Current guarantees come from the contracts above;
current readiness comes from the release assessment. Failed criteria, cache
conditions, digest subsets and provider limitations remain part of the evidence.
