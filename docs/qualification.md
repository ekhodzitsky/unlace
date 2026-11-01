# Matched qualification protocol

This protocol defines how scoped superiority claims may be earned. The
[pinned selected-audio report](matched-comparison.md) records the current limited
consumer job, results and unresolved equivalence boundaries. This protocol itself
contains no performance results. Correctness and equivalent work are
prerequisites to timing; decoding, transcoding or playback must not be timed against demuxing alone.

## Corpus and output contract

Record rights/provenance and SHA-256 for every immutable input. Include short
and multi-hour audio-only WebM, mixed-track files, multiple languages, browser
MediaRecorder streams with unknown size, absent/end-of-file Cues, every lacing
mode, nonzero delay/preroll/padding, invalid indexes and every-byte truncations.
Synthetic fixtures must be independently checkable; private recordings stay local.

For each selected track compare the ordered packet payload hashes, boundaries,
timestamps and units, duration, codec initialization and signed trimming data.
Check final completion separately from the ability to read the first packet.
Use at least two independent container implementations. When they disagree,
minimize the case and resolve it against the specification; majority agreement
is not proof. Decode qualification, when needed, uses the same decoder and
identical settings for all demuxed outputs.

## Resource and performance measurements

Pin exact competitor revisions, build flags, compiler, OS, architecture, CPU,
input hashes and all command lines before collecting results. Record actual
supported capabilities; unsupported, failed, unavailable and not tested are
different outcomes. Include at least libwebm, Nestegg, libavformat, Symphonia
and matroska-demuxer where an executable adapter is available.

Measure time to the first selected packet, end-to-end demux throughput, peak
RSS, allocation count/bytes, bytes read and seek latency. Use the same source,
track, output work and buffering policy. Distinguish cold/warm filesystem cache,
startup/index cost and steady state. Repeat timed runs, report variability,
absolute values and units, and retain unfavorable inputs. File-read savings
are not network savings. Packet timing precision is not decoded PCM accuracy.

## Robustness and ergonomics

Inject tiny/short/interrupted reads, transport errors, missing ranges, forged
sizes/offsets and resource exhaustion. Bound CPU as well as memory. Fuzz VINT,
hierarchy, lacing, timestamp arithmetic and seek state; retain minimized regressions.
Successful fuzzing is evidence for its exercised scope, not proof of safety.

Exercise complete consumer examples for sequential reading, selected audio,
seek and incomplete-input continuation. Compare allocations/copying alongside
the clarity of ownership, error recovery and source requirements. Do not turn
subjective convenience into an unexplained numerical score.

## Publication of claims

Keep raw recordings, process traces and local tracker records ignored. A sanitized
report may include corpus identities, versions, reproduction commands, aggregates
and relevant limitations. A claim names its dimension and tested scope. Passing
a small corpus never licenses a statement that every WebM is supported or that
unlace is universally better. No public release or repository visibility change
is authorized by a successful comparison.
