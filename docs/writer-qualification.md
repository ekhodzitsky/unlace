# Writer qualification

The unreleased writer is qualified for the documented [encoded WebM subset](writing.md)
by independent packet, metadata and scoped decode checks. This is not a general
Matroska conformance, arbitrary remux, playback compatibility or performance claim.
The initial per-block Cluster limitation and a laced-padding playback disagreement
remain recorded below. No codec encoding is performed by unlace.

## Counterparts and method

Pins match the [selected-audio comparison](matched-comparison.md): libwebm
`a7c6662c68a90af18dd53dbb1f4262a346e9a547` (classic full-load mkvparser and
separate incremental webm_parser), Nestegg
`767aab25013acefbdcc6d68a2a7a2c9081303e3f`, and FFmpeg 7.0.2/libavformat
61.1.100. Native adapters use GCC 15.2.0, C++11/C11 and -O3. The additional
original classic/Nestegg adapters select a requested track and expose its public
UID where available; upstream sources are unchanged. Hardware, OS and original
adapter build methods are retained in the matched report. This report measures
correctness and resource admission, not comparative speed.

Original input rights, producer pins and hashes are documented in
[packet qualification](packet-qualification.md), [Opus timing](opus-timing.md)
and [robustness](robustness.md). No competitor sources, raw media or research
adapters enter the ordinary build/test suite.

The selected-audio remux example writes live and indexed output. It preserves an
already available source TimestampScale, using exact one-nanosecond ticks if Info
follows Tracks. No clock is rounded to improve packing. A bounded line-by-line
checker compares every original packet byte, boundary/order, exact presentation
ns, whole-block signed padding, lace identity, CodecPrivate, delay/preroll and
exposed TrackNumber. Native stream ordinals are mapped explicitly to the selected
track. Original manifests require End and reject extra/missing packets. At most
32 MiB is allowed per manifest line; each reader has a 1 GiB address-space limit,
60 CPU seconds and 90 wall seconds. The checker streams the six-hour results
without retaining the full output.

All seven sources in both modes match classic libwebm, incremental libwebm and
Nestegg: 42 complete trials. Mono additionally matches libavformat in both modes.
Its native 1 ms time base cannot directly express the 6.5 ms delay. Exact ns
reconstruction is admitted only when public pre-skip/delay establishes that
fractional offset; raw trim is compared without rounding, following the matched
report. This does not claim libavformat exposes arbitrary exact clocks.

## Packed output identities

Indexed output from an unknown-size browser input proves a complete serialization
of observed packets; it does not prove the producer intended no further packets.
Defaults retain at most one bounded 1 MiB Cluster payload, closing it at one second
of block-origin span or the signed relative tick limit. Explicit flush closes it
sooner. Source precision and block/lace boundaries remain unchanged.

| Source | Mode | Output bytes | SHA-256 |
| --- | --- | --- | --- |
| browser-audio | indexed | 20806 | `978efd851304b490bad98b19cc1954c2cada9a4ac9ead52e24f0efa3d7f773e4` |
| browser-audio | live | 20557 | `d7e4a424e4dccf45af735e2be0d143748566471c305409705107d08525aee25b` |
| browser-mixed | indexed | 20806 | `815fa29639e6511dfaeef230905d950181744f4d503f0a0973914e31f9535f95` |
| browser-mixed | live | 20557 | `e846fc3f9ea3f9191ace3f60218ef24e0a6235e172603f8054f3be5f01394a51` |
| mono | indexed | 10971 | `722d83267531e68328f299fb52685716b5744dd04f5c61b2aa5ec894bd025f22` |
| mono | live | 10722 | `08efbe0909d53f99b53394426e24416ff0eb7eeb7c5fd3dc5921381445ace516` |
| six-hour | indexed | 11215597 | `a042f7f6a039898c99c6da9705cf08503cbaa4681b9377dd57f486fc511b1200` |
| six-hour | live | 9962604 | `e037068a7470861a51771fa62e942995b9a6872125d598013393980cf9a4500a` |
| stereo | indexed | 20318 | `e0ac2ae0dd485b08e07e44e9b3cea068d5042561831091eae8d35e82579da916` |
| stereo | live | 20069 | `2759e6381c80e6ecf40427791043910c0c25cce471f9f2ff1e30e304f7665d2b` |
| synthetic-vorbis | indexed | 4751 | `a08cb68d344c1db3510fcaa713a3b8f59ff5a818ceddac8298300fa3c05b41a4` |
| synthetic-vorbis | live | 4560 | `dea79c71fbcc809c8b1f5d1bf0954c29e3fe38c46d9a6dc54151ddd45d1a2adb` |
| youtube | indexed | 9770329 | `39414f29c97bad53894fbe61301d95b1993f6d634fa26d867edde860df25ec98` |
| youtube | live | 9735568 | `a50eca6caa23da78a9c031386a2c128d5a911422f2195a6d28d8a606809d070b` |

The six-hour source has 1,080,001 blocks/packets. Packing reduces its Cluster count
from 1,080,001 to 21,601 without changing its ordered packet digest
`6c0fa3b76ead4402c47bf277260d818c9c2db3630a94a0dc4df23d362664a561`.
The classic full-load reader now reaches End under the original 1 GiB allowance;
its measured peak RSS is 235,112 KiB indexed and 235,160 KiB live. These are single
resource-admission trials, not repeated performance measurements. The two
incremental counterparts also match all packets. The indexed output contains
21,601 cues; explicit cue limits can still refuse denser/longer recordings.

## Multiple tracks and encoded video

Separate own producers submit already encoded data to the public writer API in
both modes. One combines four VP9 video frames with 21 Opus packets, and another
uses four VP8 frames with 21 Opus packets. Encoded VP8 is produced externally from
the own mixed fixture; unlace only copies encoded bytes. Video dimensions are
64 by 64. Track numbers 1/2, UIDs 1001/1002, codec initialization and audio delay/
preroll are declared explicitly. Caller order merges raw clocks monotonically.
The writer does not infer video keyframes or reorder submitted media.

Another output interleaves 52 mono and 52 stereo Opus packets as tracks 1/2,
UIDs 2001/2002, retaining each stream's exact packet order, timestamps and padding.
The generated mixed files use one Cluster; dual-audio files use two. Classic
libwebm and libavformat match every audio/video payload and declared timing;
libavformat also matches the declared keyframe flags. Nestegg independently matches each selected audio track and its
private/delay/preroll/padding fields. Classic libwebm verifies configured UIDs.
These checks cover declared supported fields, not source fields absent from the
writer configuration. Both mixed video streams decode to identical complete RGB
bytes as their respective source, and both audio tracks decode to identical
complete PCM as their original source. They do not qualify arbitrary codec setup,
video random-access correctness or broader video metadata.

Fixed, Xiph and EBML lacing are each emitted in both modes as two three-frame
Opus blocks. Every frame is the valid encoded silence packet `f8fffe` (960 samples
at 48 kHz). With DefaultDuration 20 ms and BlockDuration 60 ms, the first block
has -25 ms DiscardPadding and the second +25 ms. Each trim spans more than one
frame. Classic libwebm and Nestegg match all six packet bytes and raw block trim;
libavformat public packet positions also preserve the two block groups.

The additional generated output identities are fixed below. The VP8 producer's
duplicate dual/laced cases reproduce the same bytes and are not new corpus cases.

| Case | Mode | Bytes | SHA-256 |
| --- | --- | --- | --- |
| mixed-vp9 | indexed | 7398 | `795c642902decaea9251ca5cb77b027045f920a913745b961dad22c0881c3db0` |
| mixed-vp9 | live | 7149 | `f860b89db78d2ae31e38a9fc885cb49132f54ca438013f237606cd9686055b2a` |
| mixed-vp8 | indexed | 8955 | `a4cc2462bbeec201be9508dc58e622c4e6ea417a55c30a889fb0b0722ebbff16` |
| mixed-vp8 | live | 8706 | `2b8cf25bbafd6b76e29fc5504a70d9be692ee665e26f191f438ed573eb384abf` |
| dual | indexed | 31030 | `2bd0ea4287cdb5f95d8bd7f9da9aa6a1091550dec4a86aba201dc83b2c81afbc` |
| dual | live | 30664 | `04e9e3c5fb84b2e108df1cfda83c84b947fe230074f4f3676fce51d81095c964` |
| fixed | indexed | 475 | `9dc9383ad5a783a39d43ab15a6bfea7649cca5141c17b6b99bff0a0660121327` |
| fixed | live | 284 | `556ccfa0531223353101d50290dafdaafb716c3d13796ed733b96f463db289b5` |
| xiph | indexed | 479 | `80753005aaf313f2a7e103c2ac8fdf92f8d10fca4a019061aba3809f06d2cef8` |
| xiph | live | 288 | `cf91f57e50f412c6fb7a7bcae613708c1e5fbb2b070ad6539f718278c997d16a` |
| ebml | indexed | 479 | `703bde6a60194a629e751a1eb66756fe9733e7300ba6020bd47db78be37f0e74` |
| ebml | live | 288 | `3e8a7d7280d6218d4c00d6e9953a78ab493d0af2e995fbc12d37f1ad082676a4` |

## Decoded trim and playback limitation

The pinned independent FFmpeg/libopus decoder produces identical complete float
PCM from original mono/stereo sources and both packed output modes: 49,440 sample
frames per channel, retaining the 312-sample initial skip and 168-sample final trim.
All bytes, including the first/last 64 frames, match. Mono PCM SHA-256 is
`515227b59e9202479a9de2c2e6fa5e8a03028aef728aab3b2836a9cb3954c35c`;
stereo is `72247819d5ce936db289546d471dbe0c62c2e18cc182fd8c4aea5c300e6698b2`.
The original independent trim adapter is described in [opus-timing.md](opus-timing.md).
No six-hour PCM decode or broader multichannel qualification is claimed.

The laced fixtures expose an unfavorable playback difference: the pinned FFmpeg
CLI decode returns 2,880 mono sample frames, while whole-block trimming requires
3,360. Its packet side data repeats the 1,200-sample skip/discard on every frame
of each corresponding block. This is not silently treated as correct playback.
[RFC 9559 DiscardPadding](https://www.rfc-editor.org/rfc/rfc9559.html#name-discardpadding-element)
defines padding at the beginning/end of a Block. The writer therefore preserves
the block value once rather than changing valid metadata to accommodate that
consumer behavior.

A separate original libopus 1.6.1 adapter groups the independent libavformat
packets by public AVPacket.pos, decodes all frames and trims each whole group
once. Unlace's path groups by explicit lace index/count. At 48 kHz mono s16,
no FEC and zero gain, both paths produce exactly 3,360 frames for every lace/mode,
SHA-256 `53192a12b178687771e5a607727c1a7102e095fc05515a6e827a4eeff529a280`.
This establishes block-trim agreement for the fixtures, not ordinary FFmpeg
playback agreement. The historical selected-packet benchmark checked metadata,
not playback; its timings and admission gates are unchanged.

## Failure, bounds and seek checks

Thirty writer integration tests cover supported metadata/identity, mode receipts,
all laces including empty frames and 256 slots, signed padding, wide EBML size
differences, unknown later frame times, nonrepresentable/nonmonotonic clocks,
exact cue-byte limits and verified index positions. Faults include every-byte
write failure, short/interrupted/zero/invalid writes, WouldBlock, each constructor/
finalization seek, seek mismatch, constructor/per-operation call budgets and
finalization flushes. I/O failures poison state and return no successful receipt.

Packing regressions cover owned buffered payloads, byte/time/signed-tick bounds,
oversized direct blocks between packed Clusters, flush without finish, negative
first clocks, nonfirst-block cues, and invalid preflight leaving earlier buffered
media untouched. A regression expecting two Clusters for 60 twenty-millisecond
blocks failed against the original 60-Cluster behavior before the fix. Generated
sequences compare every declared payload/time/raw clock/track/lace/padding/duration/
flag across 96 configurations in both modes, totaling 6,144 accepted blocks with
varied explicit flushes and resource refusals. This is deterministic generation,
not coverage-guided writer fuzzing or a safety proof.

An internal arithmetic regression distinguishes live unknown Segment offsets
from indexed finite-size limits. Checked u64 offsets/counters still apply;
no petabyte-scale physical I/O test is claimed. Allocation and work boundaries,
including retained capacity overhead, are documented in the writing contract.

Verified seeks on the packed six-hour indexed file at 0, 1, 3 and near 6 hours
compare every suffix packet field and requested one-second window against full
iteration. The explicit index-read budget is 500,000 calls. Repeated/backward
seeks on mixed output verify the selected audio track through real nonfirst-block
cue positions. Actual starts, preroll and fallback remain observable; neither
container seeks nor index success establishes decoder readiness.

A final-flush fault after the finite header patch leaves parseable finite bytes
but returns an error and no receipt. Generic Write + Seek cannot promise atomic
publication or durability. The staged file example separately refuses truncated
input and preserves an existing destination byte for byte; failed staging data
is retained. These outcomes do not depend on parsing partial bytes as a success.

## Retained initial limitation and reproduction

The first writer source (`3f33e94`) used one Cluster per block. With its six-hour
output, classic full-load libwebm stopped after 115,574 packets under a 1 GiB
address-space cap and 244,368 under an explicit 2 GiB diagnostic allowance. Both
modes returned adapter status 8 without End; larger-cap peak RSS was about
1,108,000 KiB. Incremental libwebm and Nestegg completed. These observations were
consistent with directory/block retention, not proof of malformed output or a
general library failure. The packing results above address that measured layout
cost while retaining the original unfavorable evidence.

Rebuild with Rust 1.89.0 and run the checked-in `remux_audio` example for each
source and both modes as shown in [writing.md](writing.md). Compare complete
native manifests with the original source manifests, requiring End, bounded
streaming equality and explicit field availability. For mixed/dual/laced output,
submit the declared tracks/blocks above through both writer types and compare
each track separately; decode with the same pinned settings on both sides.
Raw outputs, exact reproduction scripts, adapter sources/builds, process traces,
full manifests and PCM reports remain ignored under `runs/write-qualification`.
Relevant retained checks are `check_packed_corpus.py`, `check_packed_short.py`,
`check_produced.py`, `check_produced_vp8.py` and `decode_laced_groups.py`.
The earlier per-block reports remain separate; stale pre-rebuild output is excluded.
External media/tools are optional qualification infrastructure, never Cargo
dependencies or ordinary test requirements. Broader unsupported metadata and
ordinary laced-padding playback interoperability remain explicit limitations.
