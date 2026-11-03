# GigaSTT WebM integration pilot

Assessment: 2026-11-03. Unlace source/API remains **0.2.4**. Its repository is
public, with no published crate. The owner selected GigaSTT as the concrete
consumer; this is a completed isolated pilot, not an installed application change.

**No-go for a transparent replacement of the current GigaSTT WebM path.**
Packet and same-decoder checks pass on the shared subset, but admission,
trimmed PCM, truncated-source behavior and the declared independent numeric PCM
gate differ. No production unlace defect was established or parsing guarantee
relaxed. GigaSTT source and configuration were not modified.

## Frozen implementations and boundaries

The library bytes are the unchanged 0.2.4 implementation used at `eef3e14`;
subsequent source-visibility documentation commits do not alter its code.
The consumer identifies as gigastt-core 2.21.0, based on `ca661d5f642065d86cae75560dc36821823ffe0c`
with pre-existing working-tree changes. A 331-file copy was frozen locally;
its snapshot record has SHA-256
`867e4efa649d00ab5e000d8a217d111f60fe0c01a75cec84ffb02e29fee90ecf`.
That public base commit alone cannot reproduce the tested dirty consumer.

The isolated copy adds a local adapter, diagnostic examples and an append-only
first-PCM hook. Original packet decoding, mixing, resampling and inference
functions are used unchanged. The hook fills to the first available PCM and
then uses the original drain. Sixteen checks against the uninstrumented public
decode_audio_file call produce identical complete PCM. Four existing files differ
from the snapshot: Cargo.lock, the consumer manifest, audio module registration
and the appended hook. None of these modifications enters GigaSTT's working tree
or unlace's tracked library/dependency metadata.

Consumer builds use Rust 1.94.0, opus-rs 0.1.34, Symphonia 0.6.1, rubato 5.0.0
and ort/ort-sys 2.0.0-rc.13. Only file-decode and dynamic-ORT consumer features
are selected; diarization, network downloads and quantization are disabled.
ORT download/TLS features are removed in the isolated manifest to use an existing
local ONNX Runtime 1.28.0 CPU library. This is the tested native runtime, not a
claim about GigaSTT's packaged runtime. Model encoder, decoder, joiner, vocabulary,
source, executable, inputs, helpers and native runtime have retained hashes.
All third-party consumer dependencies, media, model copies, caches and raw
measurements stay ignored/local; unlace still has zero Cargo dependencies.

## Comparison lanes and fixed limits

The existing file path uses FileWindows, Symphonia packet demuxing, GigaSTT's
OpusStream and ResampleTo16k. Mono mixing is the arithmetic channel mean; Opus
decodes at 48 kHz regardless of a capture-rate hint. Resampling stages exactly
48,000 source frames. The pilot preserves that summation and staging cadence.

- Baseline: the actual GigaSTT file decoder and first-PCM hook.
- Diagnostic: unlace packets through the same decoder/resampler, deliberately
  without trim to test existing-behavior equivalence. This is not the candidate
  production policy and does not erase or reinterpret parser metadata.
- Strict: unlace packets with one whole decoded block retained for signed
  DiscardPadding, then initial Opus pre-skip removed once, before the same
  resampler. No encoded byte or timestamp is rewritten.

The strict profile requires one enabled mono/stereo A_OPUS track, zero OpusHead
gain and a contiguous determinate packet timeline within one millisecond of
container clock quantization. It refuses gaps, ambiguous timing and multiple
candidate audio tracks rather than silently choosing or filling. Padding is
rounded to the nearest 48 kHz frame only when its representation differs by at
most one nanosecond; other fractional-sample padding refuses. These are local
adapter admission rules, not new library restrictions or Matroska conformance rules.
All tested heads use mapping family 0; other mappings and multistream decoding
were not qualified.

Limits recorded before final execution are 64 MiB encoded input, 125 decoded
source seconds, 64 KiB packet/private bytes and one million polls. Decode processes
have 1 GiB address space, 512 MiB peak-RSS criterion, 30-second wall supervision
and a 250 ms first-PCM criterion. The Rust clock starts after the file-size check
and includes opening/probing, decode and resampling. Process wall time separately
includes startup, output fingerprints and file writes; no HTTP or first-transcript
latency guarantee follows. The local adapter retains complete diagnostic PCM,
so its memory observation is not a bounded-window implementation for arbitrary
recording length. GigaSTT's deployed file-window path remains a separate integration.

## Packet, trim and failure results

The final correctness cohort contains 27 inputs and 108 application runs:
16 nominal inputs, five truncations, four other profile refusals and two signed
whole-lace-padding controls. Nominal inputs include prior mono/stereo, own
English/Russian/German sources in original/live/indexed forms, 120 seconds of
qualified long speech, three existing consumer fixtures and one original valid
three-frame laced control.

| Gate | Observation |
| --- | --- |
| Shared nominal admission | 14 of 16; two GigaSTT-admitted fixtures are refused by unlace. |
| Ordered packet payloads and boundaries | All 14 shared cases match the separate Symphonia traversal. |
| Untrimmed diagnostic PCM | All 14 match the actual baseline byte for byte at 16 kHz; raw 48 kHz samples also match. |
| Strict PCM versus existing baseline | All 14 differ after applying trim. This change is retained, not normalized away. |
| Strict span versus successful native decode | All 14 have the expected complete 48 kHz sample count. |
| Exact same-decoder crop | All 14 plus two signed whole-lace controls match independently selected spans of the baseline raw samples. |
| Original public consumer call | All 16 nominal files match the instrumented baseline, including files unlace refuses. |

The two admission differences follow documented current unlace checks. One
consumer fixture declares 48,000 Hz in OpusHead but 16,000 Hz in Audio
SamplingFrequency, returning InputRateMismatch. Another declares seven
milliseconds of BlockDuration but 53.5 milliseconds of DiscardPadding, returning
PaddingExceedsDuration. Native acceptance does not justify silently changing
these [timing contracts](opus-timing.md) or establish a parser defect.

Three finite-file truncations refuse in both paths. Two late live-WebM payload
truncations instead produce successful partial PCM in GigaSTT, through its
UnexpectedEof-after-audio policy; unlace returns located truncation errors and
no successful PCM result. An unknown-size source's observed clean boundary still
cannot prove intended source completeness. Early and after-three-packet adapter
cancellation both refuse without a successful PCM output. These cooperative
checks do not interrupt a blocking provider or decoder call.

Multiple audio tracks, a gap, nonzero gain and nonrepresentable fractional
padding are explicit strict-profile refusals. Positive and negative padding
spanning more than one lace produce the expected 1,128 source frames, rather
than trimming the last/first packet independently. Four isolated Rust 1.89 tests
also check signed whole-lace trim, pre-skip across blocks, alignment and excessive
padding. Ordinary native laced-padding playback remains separately limited in
[writer qualification](writer-qualification.md); it is not the whole-lace oracle.

## Independent numeric PCM and recognition

Equal length alone is not sample fidelity. A separately pinned FFmpeg 7.0.2
reference explicitly selects its libopus decoder, emits mono 48 kHz f32 and uses
an arithmetic-mean stereo mix to match GigaSTT. The declared thresholds are
normalized RMSE at most 0.0001 and maximum absolute sample error at most 0.001.
**Zero of 14 shared cases pass that numeric gate.** Normalized RMSE ranges from
0.000228 to 0.491420. The thresholds are not an Opus conformance definition, a
perceptual quality measure or proof of a decoder bug. The exact packet/raw-PCM
checks locate the comparison beyond a changed demux payload; they do not explain
the remaining independent-decoder differences. No sample-fidelity approval is claimed.

Initial numerical tooling used FFmpeg's automatic native decoder and default
stereo remix. Those observations are preserved as non-equivalent work. The
corrected explicit-libopus/mean reference was replayed with helper, candidate PCM,
input and executable hashes checked before/after; thresholds were not raised.
A known-passing identical-PCM control accepts, while amplitude corruption,
missing pre-skip span and a failed native exit are rejected. The first exploratory
adapter also omitted the gain constraint and full pin checks; review found these
and the final cohort was rerun with explicit refusals and immutable input/model/
runtime verification. Earlier results remain distinct.

Recognition uses the existing local INT8 GigaAM v3 RNN-T triplet, one pool slot,
one encoder thread, CPU execution, no VAD/diarization/hotwords/punctuation/ITN.
The same Samples API compares decoded inputs; actual file API calls remain a
separate lane. Model loading and inference are reported separately. The first
load creates an optimized graph cache within the local copied model directory;
subsequent runs can reuse it. Cache-state timings are not pooled into a claimed
universal startup latency.

There are 29 model processes. Twelve cover one Russian synthetic utterance in
three container forms and four lanes: file/baseline/diagnostic return empty text;
strict returns one word. Native-decoded and original-WAV controls also return
that word. This is weak quality evidence and remains a visible lexical change.
A separate preselected extension tests three existing natural Russian consumer
fixtures, encoded to 64 kbit/s WebM/Opus: original WAV, actual file API, baseline
Samples, diagnostic Samples and strict Samples all produce the same text with
zero errors against **14 reference words total**. Word errors use lowercase,
alphanumeric tokens, yo-to-ye normalization and Levenshtein distance. These
15 observations are a smoke check, not statistical WER non-regression or a
quality ranking; duration, confidence and word timing need not stay identical.
No REST, SSE, WebSocket, VAD, diarization or deployment rollout is qualified.

## Timing and resources

The fixed timing cohort is 252 fresh processes: six shared inputs, three lanes,
seven alternating-order repetitions, warm and advisory-evicted filesystem data.
It includes 1.03-second mono/stereo, the three Russian container forms and
120-second speech. Two allowed CPUs and MALLOC_ARENA_MAX=1 are fixed. Cache advice
is not proof of physical cold storage. No heavy pilot job ran concurrently, but
the shared host is not an isolated benchmark environment.

| Lane | Processes | First PCM max (ms) | Decode max (ms) | Process wall max (ms) | Peak RSS max (KiB) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Existing GigaSTT | 84 | 11.379 | 271.704 | 296.523 | 15,044 |
| Untrimmed diagnostic | 84 | 11.349 | 293.773 | 350.513 | 36,960 |
| Strict adapter | 84 | 12.728 | 290.799 | 346.405 | 36,748 |

All 252 return successfully, match their complete correctness records and meet
the fixed limits. Extra raw-PCM retention, strict validation and different trim
work preclude a general speed or memory-superiority claim. All 29 model
processes meet their separate 4 GiB address-space, 1 GiB RSS and 90-second wall
limits; observed maxima are 896,744 KiB RSS, 1.738 seconds whole-process wall
and 1.205 seconds model loading. The first model-cache construction dominates
peak memory. These are separate from decode-only measurements.

This sequential pilot does not construct a seek index. Its passing first-PCM
criterion does not erase the earlier failed 250 ms cold prepared-seek setup in
[the prepared-session report](prepared-sessions-0.2.4.md).

## Decision and verification

The container-to-existing-codec mapping is supported for the tested admitted
subset, including explicit trim and failure policies. A transparent GigaSTT
replacement is not approved: the application must deliberately adopt trim,
track/timeline/gain admission and truncation behavior, establish its accepted
PCM/decoder oracle and test the actual file-window/service integration. The
bounded 125-second adapter is not a deployed long-form consumer. These are
concrete adoption differences, not a requirement to implement every Matroska feature.

Independent final review passes 63 aggregate, table, source and pin checks and
checks all 252 timing rows individually. The positive/corrupted numeric controls
and natural-reference texts were independently rechecked; no required correction
remains. This is scoped same-session review, not external certification.

All six unlace Rust 1.89 gates pass again, with unchanged library, test, example,
Cargo and dependency hashes. No source/API version bump is due; qualification
is recorded under Unreleased. Raw consumers, models, media and measurements stay
local under the [evidence policy](../agents/local-only-files.md). Public source
visibility does not authorize crate or release-artifact publication.
