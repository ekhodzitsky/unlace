# Release and production readiness

Evidence assessed through 2026-11-02; documentation reviewed 2026-11-03.
Current source: **0.2.4, public repository, unpublished crate**, under MIT. Tests and local
qualification cover Linux x86_64 with Rust 1.89.0. This page states the current
decision; dated methods and observations remain in the [evidence index](README.md).

**Scoped go for the fixed prepared local consumer. General production readiness
is not established. Crate-release prerequisites remain.**

## Public distribution

| Gate | Current disposition |
| --- | --- |
| License | MIT is selected and recorded in LICENSE and Cargo metadata. |
| Source coherence | Cargo.toml and Cargo.lock agree on 0.2.4; contracts, examples and changelog describe the supported subset. [Version policy](versioning.md). |
| Local verification and packaging | Required Rust 1.89 checks, dependency audit and local package construction/verification pass. [Verification procedure](verification.md). No hosted workflow is required or authorized. |
| Supported scope and compatibility | The [experimental policy](experimental-scope.md) defines the supported subset, pre-1.0 compatibility and private reporting route. |
| Public reporting contact | Not yet established. The owner must name a monitored contact before a packaged release; public source access is not a support SLA. |
| Source visibility | Owner authorized public repository visibility on 2026-11-03 after a reachable-history audit. |
| Package publication | Not authorized. Cargo publish=false; no public crate, tag or release artifact upload is authorized. |

Public source is available for the documented experimental subset. The technical
evidence does not imply a published crate, application acceptance or general
Matroska support. Package-release prerequisites remain separate.

## Production decision

| Scenario | Evidence and disposition |
| --- | --- |
| Prepared local encoded-packet consumer | The [bounded concurrent pilot](consumer-pilot-0.2.4.md) passes: 28 processes, 7,168 attempts, 3,964 full-field matching admitted windows, 3,204 explicit Busy refusals and zero admitted deadline/fidelity failures. Scoped go only for its fixed source/request/resource profile. |
| New source requiring fully prepared seek state within 250 ms | Not met. All fourteen advisory-evicted initial observations in the [prepared-session evaluation](prepared-sessions-0.2.4.md) fail. Full preparation costs seconds and remains visible. |
| Arbitrary provider with hard cancellation deadline | Not qualified. Synchronous provider calls can block; the concurrent pilot retains a cancellation miss and an externally terminated never-returning provider. |
| GigaSTT sequential WebM/Opus | [Completed isolated pilot](gigastt-integration.md): shared packets and same-decoder crops pass, 252 decode processes meet limits and three natural clips preserve text. No-go for transparent replacement: admission, trim and truncation semantics differ; the independent numerical PCM gate fails. A [53-byte packet follow-up](gigastt-integration.md#packet-level-localization) reproduces the codec-output difference before container transforms. No deployed service approval. |

The passing local profile retains each original immutable provider, performs
explicit full preparation before service, fixes selection, admits at most one
request per source and uses exclusive sessions. It neither transfers verified
state across independent providers nor persists it through process restart.
The 44.7% Busy rate in the burst cohort is an explicit no-queue tradeoff, not
successful service for every caller. Preparation and provider blocking are
outside the prepared-request timing claim.

## Retained limits and evidence

- [Container contracts](README.md#current-contracts) define supported structure,
  exact timing/trim, explicit refusals, completeness and resource limits. Decoding,
  networking, source immutability and application buffering remain consumer duties.
- [Source audit](source-audit.md), [robustness](robustness.md), [writer campaigns](writer-fuzzing.md)
  and [same-session review](independent-review.md) cover stated regressions and
  bounded controls. They are not exhaustive hostile-input, allocator or platform proofs.
- [Writer qualification](writer-qualification.md) retains the ordinary FFmpeg
  laced-padding playback disagreement. Checked finalization cannot retract bytes
  after generic failed flushes or establish directory crash durability.
- [Performance reports](README.md#performance-and-competitors) preserve slower
  cases, first-use misses and retained storage. No overall superiority follows.
  Prepared ownership adds a lifecycle over already-retained verification, not a
  new faster parsing algorithm.
- [Historical reference pilots](reference-pilot.md) retain earlier refusals and
  deadline misses. Later scoped passes do not erase those observations or turn
  simulated providers into a real network service.

## Before a stronger claim

The next production gate is explicit GigaSTT adoption of the [measured trim,
admission, PCM and failure policies](gigastt-integration.md#decision-and-verification),
followed by its actual file-window/service integration. Qualify the real workload,
platform, provider identity/latency contract, admission/retry and shutdown policy,
resource ceilings and completion/durability requirements. Qualify that integration
against explicit acceptance criteria, retaining setup cost and refusals. Use the
completed local pilot as a baseline. Further assurance and optimization should
follow that risk profile; beating every competitor or implementing all Matroska
features is not an invented prerequisite for a defined supported consumer.

Crate publication separately requires the owner decisions above. Public
repository access does not authorize a package release or general production claim.
