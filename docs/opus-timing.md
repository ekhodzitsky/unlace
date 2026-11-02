# Opus initialization, delay and block trimming

Unlace validates Opus identification metadata for A_OPUS tracks before emitting
TracksReady or packets. It still does not decode audio, parse Opus audio packet
contents, apply gain, resample or remove samples. CodecPrivate and packet bytes
remain unmodified. Other codec IDs remain container-level opaque data.

## Identification metadata

`opus::parse_opus_head` returns borrowed, immutable OpusHead metadata without
allocations. `Track::opus_head()` provides the same checked view of A_OPUS
CodecPrivate. The parser checks signature, required lengths, compatible major
version, channel counts, stream/coupled counts and mapping indexes. It preserves
pre-skip as exact u16 samples per channel at 48,000 Hz, input rate as informational
u32 hertz (zero means unspecified), and output gain as signed 1/256 dB units.
Input rate does not instruct a consumer to choose a decoder output frequency.

Mapping families 0, 1 and 255 are supported. Family 0 has one/two channels and
an implicit map. Family 1 permits up to eight output channels; family 255 permits
up to 255. Explicit stream counts must be nonzero, coupled count cannot exceed
stream count, and their sum cannot exceed 255. Map indexes must reference a
decoded channel or use 255 for silence. Duplicate/silent indexes are permitted.
Other mapping families return an explicit unsupported kind rather than assuming
they share these structures. Versions below 16 have a compatible major version;
additional bytes are retained for minor versions above 1. Version 0/1 trailing
bytes are rejected. These rules follow [RFC 7845 section 5.1](https://www.rfc-editor.org/rfc/rfc7845.html#section-5.1).
No playback or multichannel decoder support is implied by metadata parsing.

For A_OPUS, the track must be audio with a present valid CodecPrivate. Audio
Channels and SamplingFrequency must match the header, as required by the
[Matroska codec mapping](https://www.matroska.org/technical/codec_specs.html).
Standalone header parsing accepts unspecified input rate zero. The integrated
reader applies the literal mapping equality to positive SamplingFrequency, so
that combination reports InputRateMismatch instead of inventing a rate. Raw
header inspection remains available for diagnosing such producer metadata.

## Exact pre-skip and integer CodecDelay

`Track::opus_timing` exposes validated pre-skip and its relation to CodecDelay.
The exact pre-skip time is `pre_skip_samples * 1,000,000,000 / 48,000` nanoseconds.
The API returns the numerator and documents denominator 48,000; it does not
round this through floating point. One sample is 62,500/3 nanoseconds and cannot
be represented exactly as an integer nanosecond CodecDelay.

The mapping specifies that ratio without prescribing integer rounding direction.
Unlace accepts either adjacent integer nanosecond value and exposes the outcome
as `DelayPrecision::Exact` or `Quantized { difference }`. The difference is
`CodecDelay * 48,000 - pre_skip_samples * 1,000,000,000`, in units of 1/48,000 ns.
A nonzero difference is always reported, never labeled exact or used to rewrite
the raw delay/timestamp. Any difference of one nanosecond or more is a typed
CodecDelayMismatch. This explicit quantization rule is the reader's policy where
the mapping leaves integer rounding unspecified; it is not a stronger claim
about the precision of the source. An unwritten zero CodecDelay uses EBML's
semantic zero default; it matches only zero pre-skip. Missing nonzero delay
therefore fails instead of being silently derived from CodecPrivate. The
[codec mapping](https://www.matroska.org/technical/codec_specs.html#a_opus) uses
mandatory-presence wording; the reader retains its explicit zero-default policy
for native MediaRecorder compatibility. This is not a claim that physically
omitted delay satisfies every mapping/conformance interpretation.

`packet.timestamp_ns` subtracts the raw CodecDelay exactly once from the raw
scaled block origin. `block_timestamp_ns` preserves that origin before delay.
A decoder consumer removes initial pre-skip from decoded samples once. These
are clock and sample operations representing the same intrinsic delay: do not
also turn CodecDelay into another sample skip, subtract it from timestamp_ns
again, or discard more samples merely because the first packet timestamp is
negative. Pre-skip is not applied to every packet or every laced frame.

SeekPreRoll remains the raw unsigned nanosecond value. The mapping recommends
80,000,000 ns; this recommendation is not a mandatory rejection rule or an
invented replacement for a missing/zero value. Decoder state recovery on seeking
is separate work and is not proved by the sequential timing contract.

## Whole-block DiscardPadding

`packet.block_discard_padding_ns` is the original signed whole-block padding on
every laced frame. Positive values discard from the block end and negative values
from its beginning. It can span multiple laced frames. Apply it once per block,
with decoded buffering as needed, rather than independently to each frame.

`discard_padding_ns` remains a once-per-block notification, reported on the last
frame for positive padding and the first for negative padding, zero elsewhere.
That notification does not mean the padding applies only to that frame. Use the
whole-block field and lace index/count/block offset for a complete association.
No encoded packet or byte is removed, even when an entire decoded frame would
be discarded. A late BlockGroup padding field is known before its first packet.

Signed widths and duplicate fields are checked. Padding magnitude exceeding an
explicit BlockDuration, or a whole-block duration derived from DefaultDuration,
is refused. When neither exists, the container core cannot prove a decoded
sample-duration bound without interpreting codec packets. It preserves the raw
padding and unknown duration rather than claiming that bound has been validated.
Conversion of arbitrary nanoseconds to decoded samples belongs to the consumer;
no silent sample rounding or packet trimming occurs here.

## Errors and API impact

`DemuxError::Timing(TimingError)` reports typed malformed/conflicting timing or
initialization metadata. Its element ID and absolute source offset identify the
field header. OpusHead syntax identifies the exact invalid payload byte or first
missing byte; absent fields use their enclosing TrackEntry/Audio header. Causes
include InvalidScalar, DuplicateElement, missing/unsupported OpusHead, channel/
rate/delay conflicts, arithmetic overflow and padding beyond a known duration.
Fatal errors poison continuation; WouldBlock still retains state exactly.

Track literals now need `opus_timing` (normally None for manually constructed
opaque metadata). Packet literals need `block_discard_padding_ns`. Exhaustive
DemuxError matches need the Timing variant. Timing errors formerly reported as
generic Invalid now use the located typed variant. These incompatible additions are included in the private 0.2.0 source milestone.

## Decoded sample qualification

Two original 1.03-second, 48 kHz recordings, mono and stereo, were encoded with
FFmpeg 7.0.2-static/libopus. Each has 52 packets, 312 initial pre-skip samples and
3,500,000 ns (168 samples) of final padding. The unlace manifest path and an
independent libavformat 61.1.100 WebM reference path use the same libopus 1.6.1
48 kHz signed-16-bit decoder, gain zero and no FEC. The reference takes pre-skip
from its own ffprobe stream extradata and final discard sample counts from its
packet side data. It does not reuse unlace's trimming calculation.

Both paths produce exactly 49,440 sample frames per channel. Every PCM byte is
compared from local output files; complete output hashes and hashes of the
first/last 64 sample frames also agree. Encoded packet spans/counts are checked
before decoding. The adapter buffers only a bounded decoded block/reference
packet and rolling sample spans; ordinary Cargo builds/tests never use it.

| Source | Input SHA-256 | Decoded PCM SHA-256 |
| --- | --- | --- |
| Mono 997 Hz sine | `12c21d56900699e4e3c9fcde535dd8003c4f9555930a8e5f1a3dabd1ee8fd630` | `6a617d1e1bbb79de2f27ada045dd9e90ba3ce880ec9414b14f6b2d9c9b7749e6` |
| Stereo 997/557 Hz signals | `aea4850fd2710dd52d141a2d47e4ba09f45d3c71203065b213af51627417ba3f` | `ee542a936d7f5dd73622a4c0b3ff2f0040f4b030c28980f8a3bd163506ec7b28` |

The mono source uses `sine=frequency=997:sample_rate=48000:duration=1.03`.
Stereo uses `aevalsrc=sin(2*PI*997*t)|0.5*sin(2*PI*557*t):s=48000:d=1.03`.
Encode with `-c:a libopus`, inspect independent packet bytes/side data with
`ffprobe -show_packets -show_data -show_data_hash sha256 -of xml`, and initialize
the shared external decoder from each path's own metadata. A new generation may
have different random container IDs and needs a new source hash; raw artifacts,
adapters, commands and full first/last span hashes remain local and ignored.

An unfavorable attempted reference is retained: this build's default WebM-to-Ogg
`-c:a copy` path yields 49,608 decoded mono frames, versus 49,440 from the source.
That stream-copy configuration was not used as a successful trim oracle, and its
extra 168 samples were not normalized away. This is a scoped observation about
the stated version/configuration, not a claim about every FFmpeg operation.

The qualification covers these gain-zero, family-0 mono/stereo streams with
exactly representable positive end padding. Negative block-padding association
and padding spanning laces have metadata regressions, not an independent decoded
sample qualification claim. Other mappings/gains, arbitrary fractional sample
trims, multi-hour decoding, seek/preroll reconstruction, sustained fuzzing and
performance remain outside this evidence. No production-readiness or superiority
claim follows from passing these cases.
