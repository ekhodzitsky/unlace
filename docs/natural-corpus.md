# Natural speech and large container qualification

This record extends the small producer fixtures and six-hour silence checks for
private unlace 0.2.0. Container processing and codec decoding are separate jobs;
external tools used here are local research tools, not Cargo dependencies or
consumer requirements. Qualification runs on the recorded Linux x86_64 host.

## Provenance and transformations

The speech source is [LibriSpeech dev-clean, OpenSLR SLR12](https://www.openslr.org/12/),
credited to Vassil Panayotov, Guoguo Chen, Daniel Povey and Sanjeev Khudanpur, derived
from LibriVox recordings. Its archive license states copyright 2014 Vassil Panayotov
and [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Attribution, source
and license are retained locally. No recordings, transcript archives, generated
media or raw packet/PCM results are distributed in this repository.

The 337,926,286-byte dev-clean archive matches the
[published MD5](https://www.openslr.org/resources/12/md5sum.txt)
42e2234ba48799c1f50f24a7926300a1. Its SHA-256 is
76f87d090650617fca0cac8f88b9416e0ebf80350acb97b343a85fa903728ab3.
Extraction rejects links, traversal, unexpected roots, excessive file counts and
aggregate/member sizes. Ordered per-utterance hashes and FLAC sample counts are
retained locally before transformation.

There are 2,703 utterances from 40 speakers, 310,337,932 mono samples at 16 kHz,
19,396.12075 seconds (5.3878 hours). The corpus contains read English audiobook
speech, segmented by its producer. Concatenating sorted utterances introduces
artificial speaker/chapter boundaries and is not a continuous real conversation,
field recording or live browser producer. It nevertheless replaces repeated
silence/TTS as the sole long-content test.

FFmpeg 7.0.2 resamples the ordered source to 48 kHz and encodes mono Opus at a
32 kbit/s target with 20 ms frames. A second audio source is decoded, upmixed and
re-encoded as stereo Opus at a 48 kbit/s target; it has distinct packet bytes and
initialization. This is derived stereo, not an original spatial recording. Codec
quality is not measured, and decoded Opus is not expected to equal source FLAC.

The physically large mixed container combines both audio sources with an original
deterministic 256 by 256 noise image encoded as a lossless VP9 keyframe and
repeated at two frames per second. The noise generator seed is 6110214. Repeated
video makes the file large while preserving a small, known codec fixture. It is
not evidence for arbitrary natural video, inter-frame dependency chains or video
decoding. Files contain actual encoded bytes rather than sparse holes or a huge
Void element. Producer commands and binary versions stay in the local corpus manifest.
FFmpeg shifts packet clocks during remuxing. Its shortest-input mux also omits
one final mono packet in this mixed fixture (969,806 versus 969,807 standalone),
leaving no final mono padding there; the other audio retains its final trim.
This external fixture construction is recorded, not treated as a source-preserving
remux or an unlace loss. Standalone full speech files separately qualify their ends.

| Input | Bytes | Selected packets per track | SHA-256 |
| --- | ---: | ---: | --- |
| speech-mono.webm | 83,030,039 | 969,807 | 4aa9177c9a5bbe29dfc3a1d82690ef12edb99033e1dc31df37de1269d52b3081 |
| speech-stereo.webm | 121,120,901 | 969,807 | 5fdc9d5b54a50316f5e062adbee605c0d7df3e5d44c1445b5ce1e40804bc6e74 |
| speech-mixed-large.webm | 2,920,934,170 | 969,806 / 969,807 | 4c8a095bc0b9ac29c3036de36bdab30cfa090daa0ba85f907f55ee5a5f19861b |

## Complete encoded checks

The checker runs current unlace and pinned independent readers as separate
bounded processes, comparing one JSON packet at a time through each final End
and successful process exit. It retains rolling hashes and first/last summaries,
not a recording-sized packet list. Comparisons include exact selected bytes,
normalized timestamps, lacing, whole-block padding, initialization, delay and
preroll; classic libwebm also checks TrackNumber and UID. The mixed container
selects each audio track separately. Video is skipped, not decoded.

Native whole-block padding is compared with unlace's block field. It is never
mistaken for padding to apply independently to every lace. Native End means its
adapter reached the end of traversal; it does not establish identical structural
validation guarantees. Unlace must independently report finite Segment completion.
Upstream library pins and adapter semantics are those in the
[0.2.0 performance report](performance-0.2.md). These manifest checks are
correctness jobs, not performance benchmarks.

Child allowances are 1 GiB address space, 180 CPU seconds and 300 wall seconds
per comparison. The Python parent reads at most a 1 MiB line from each stream.
OS page cache, external allocator internals and total corpus disk space are
outside those process limits. Peak RSS is recorded separately from elapsed time;
concurrent encoding/review and desktop activity preclude ranking speed from it.

## Decoded and continuation checks

Independent libavformat packet/private/skip/discard metadata is streamed through
XML and decoded with the same external libopus decoder as unlace's packet path.
The reference obtains pre-skip/end discard sample counts from libavformat, while
the unlace path uses its validated Opus header and whole-block padding. Encoded
packet hashes/counts must first agree. Full sequential PCM frame counts and
rolling SHA-256, plus first/last 64-frame hashes, must match. Only one bounded
packet/block and rolling spans are retained; no full PCM/XML dump is stored.
Both standalone files matched their independent reference for all 969,807
packets and 931,013,796 decoded frames per channel, including 612 final discarded
samples. Decoder version is libopus 1.6.1, rate 48 kHz, signed 16-bit PCM, zero gain
and no FEC. Full PCM digests are:

| Audio | SHA-256 |
| --- | --- |
| speech-mono | 57ffb32b662267c203588de4fd734fec07340997411092f009baf3f59da5882f |
| speech-stereo | 6b0060a81dff8bec51c3480778c0ae17a0f8652e7de2ac9163a4b0bfb98f9ee5 |

First/last 64-frame hashes and independently derived skip/discard sample counts
also agree. This tests packet/trim fidelity with a common decoder, not an
independent codec implementation or source-FLAC quality comparison.

The same current manifest emitter is also exercised with 17-byte reads alternating
with WouldBlock, retaining one demuxer through all pauses. Every emitted packet
field and terminal record must match ordinary sequential input. A policy probe
uses deliberately insufficient payload/track limits and verifies typed resource
refusal followed by latched Failed with no further source I/O. Refusal is not
corruption, truncation or successful completion.

Seek checks compare actual forward/backward suffixes with a full sequential walk,
including bytes, block offsets, timing, flags, trim and observed End. Explicitly
raised qualification allowances remain distinct from public defaults. Provider
immutability is supplied by local files; network latency, revision races and a
real ranged/cache implementation require a consumer pilot.

## Evidence status and boundaries

All ten native selected-track jobs completed and matched: classic libwebm,
Nestegg and incremental libwebm for each standalone source, and classic/Nestegg
for each mixed audio track. This is 9,698,068 packet comparisons. Unlace peak RSS
in these correctness jobs was 2,132–2,472 KiB; native RSS and every run remain in
local records. RSS is an observed whole-process value, not a promised application
memory budget or matched performance ranking.

Three complete 17-byte/paused manifest comparisons passed: standalone mono and
both mixed audio tracks. Three restrictive payload/track policy probes refused
before exposing a packet, then latched Failed without further source I/O.
Eight forward/backward seek suffix jobs passed (four each on standalone mono and
mixed selected-first-audio) with explicit max_calls=500000, max_bytes=16 MiB and
max_scan_headers=1000000. These are local immutable buffered-file providers;
they do not validate a real network cache. With the actual public default max_calls=65536, the standalone mono before-first
request also completed; the mixed source refused with Source(Limit) before a
suffix was committed. The successful larger-budget job is not presented as
passing under defaults. Public defaults are unchanged.
These fixtures do not define application SLAs, a supported platform matrix,
multichannel decoding, arbitrary metadata/codec compatibility or general
production readiness. See the [readiness assessment](release-readiness.md).
