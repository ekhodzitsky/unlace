# Initial writer qualification

This is a partial qualification of the unreleased writer, not completion of the
writing milestone or a general WebM interoperability claim. The one-block-per-
Cluster layout has a long-file resource cost that remains to be addressed before
closing writer qualification. The [writing contract](writing.md) describes the
current API, supported metadata and output/failure guarantees.

## Counterparts and method

Pins match the [selected-audio comparison](matched-comparison.md): libwebm
`a7c6662c68a90af18dd53dbb1f4262a346e9a547` (classic full-load mkvparser and
separate incremental webm_parser), Nestegg
`767aab25013acefbdcc6d68a2a7a2c9081303e3f`, and FFmpeg 7.0.2/libavformat
61.1.100. Original input rights, producer pins and hashes are documented in
[packet qualification](packet-qualification.md), [Opus timing](opus-timing.md)
and [robustness](robustness.md). No competitor sources or media are copied into
the ordinary build/test suite.

The selected-audio remux example writes both live and indexed output without
codec encoding. A bounded line-by-line checker compares every original packet
byte, boundary/order, exact presentation ns, whole-block signed padding, lace
identity, CodecPrivate, delay/preroll and exposed TrackNumber. Native indexes
are stream ordinals; the output contains only the selected audio track. The
original full manifests require End and reject extra/missing frames. Output is
not decoded by the writer. Its one-nanosecond tick choice avoids source-clock
rounding, including after Info follows Tracks.

Mono output passes four independent reader adapters (three libraries, with both
libwebm APIs) in both modes: eight successful full comparisons of 52 packets.
The further six sources and two output modes yield 36 counterpart trials using
Nestegg and both libwebm adapters: 34 match completely, while two classic
libwebm long-file runs fail to obtain native End under the 1 GiB address-space
allowance. Incremental libwebm and Nestegg match all 1,080,001 six-hour packets in
both modes. Their ordered length-plus-packet-SHA-256 digest remains
`6c0fa3b76ead4402c47bf277260d818c9c2db3630a94a0dc4df23d362664a561`.
These digests supplement exact streaming equality; they do not replace it.

## Current output identities

All outputs below are original remuxes of the licensed/own source corpus.
Indexed output from an unknown-size browser input is a complete serialization
of observed packets; it does not prove the producer intended no further packets.

| Source | Mode | Output bytes | SHA-256 |
| --- | --- | --- | --- |
| mono | indexed | 11571 | `11354d3a090b548b022ada95e2c24e8f241c2c98b8c0c59108d6e2374d0a0830` |
| mono | live | 11322 | `29e8936cf773af71e7b27fa71f07d9dbefb90d0af2d47856150c8e5115243e17` |
| stereo | indexed | 20917 | `68e69dfbcdc571c20b970795a92ef3bb3b64964720e9be320812d15d8c498e47` |
| stereo | live | 20668 | `6b83ec09e037d0289f43b45f8296c33eee479aa2eeb2bba6e39aaf6b1504c4dc` |
| browser-audio | indexed | 21118 | `50aa9473cefe41b2c861d303fd336b25a62d1375ba3b0d7f81da33f690a469d9` |
| browser-audio | live | 20869 | `6b311df58860131ec508e80312f446d51c57808d1a0106c32bbf54174d56dfd8` |
| browser-mixed | indexed | 21121 | `f8f5bcac165184271aca20b4d134641845b6304dfc9e59235fa20cb84e3f6ae6` |
| browser-mixed | live | 20872 | `0cd925903b2a8361b33e3c912450c865ed5ab8223d4dcef3866517ecbcac8a44` |
| youtube | indexed | 10151026 | `919642ccf895696d19162a273b261c17c6e4d9d75dfab2b9d8c9c72e91fdefb6` |
| youtube | live | 10116265 | `31602fb6b9bea7e72e416a45479b055c88e336a1696f313fb2693f9c0fcd297d` |
| synthetic-vorbis | indexed | 4966 | `8ba01aabe438035818c2d2202ab277bcac49c77c62da077ba3b620e3439aebf3` |
| synthetic-vorbis | live | 4775 | `a9ae842d6e5ebddb0ff54147f6595416af3023ae20b2fb4fce20a4c0e108485a` |
| six-hour | indexed | 24958051 | `9d37e93a80614aac79e43885a962ae5cd066932ef4efeaad455ff75571a40cf3` |
| six-hour | live | 23705058 | `0d17dbfb9c278b76117a6cca23abc003c17eb05db4ccf1ce865a7aa41787af3a` |

## Decoded trim and errors

The same pinned independent FFmpeg/libopus decoder produces identical complete
float PCM from original mono/stereo sources and both output modes. Each is 49,440
sample frames per channel; the 312-sample initial skip and 168-sample final trim
are preserved. All decoded bytes, including the first/last 64 frames, match.
Mono PCM SHA-256 is
`515227b59e9202479a9de2c2e6fa5e8a03028aef728aab3b2836a9cb3954c35c`;
stereo is `72247819d5ce936db289546d471dbe0c62c2e18cc182fd8c4aea5c300e6698b2`.
The separate original read qualification uses the independent trim adapter
already described in [opus-timing.md](opus-timing.md). No six-hour PCM decode or
broader multichannel/video decoder qualification is claimed here.

Twenty-two writer integration tests currently cover mode receipts, supported
metadata/identity, enabled selection, signed trim, all laces including empty
frames and 256 slots, wide EBML size differences, unknown later frame times,
nonrepresentable/nonmonotonic clocks, exact cue-byte limits, verified index
positions, every-byte write failure, short/interrupted/zero/invalid writes,
WouldBlock, seek mismatch, constructor and per-operation call budgets, typed
unsupported Opus initialization, poison state and finalization flush failures.
Regressions preceded fixes for constructor call accounting, invalid single-frame
lace acceptance, metadata work bounds, lost Opus causes and resource/input error
classification. An original counting sink exercises six hours without retained
media or live index history; that alone is not a decoded six-hour proof.

A final-flush fault after the finite header patch leaves parseable finite bytes
but returns an error and no receipt. This demonstrates why generic Write + Seek
cannot promise atomic publication or durability. The staged file example is
checked separately: a truncated input publishes no destination, and an existing
destination remains byte-for-byte unchanged. Failed staging data is retained
for inspection rather than silently repaired or deleted.

## Long-file layout limitation and remaining work

The current six-hour output has 1,080,001 Clusters. Under a 1 GiB process address-
space cap the classic full-load adapter stops after 115,574 packets; under an
explicit 2 GiB diagnostic allowance it stops after 244,368. Both modes return
adapter status 8 without final End. Peak RSS in the larger-cap trials is about
1,108,000 KiB. These observations are consistent with directory/block retention;
they do not establish a general library failure or malformed output. The two
incremental counterparts complete under the original 1 GiB allowance.

Bounded multi-block Cluster packing, independent mixed-video/multiple-audio
output checks, wider finalization fault coverage and complete writer resource
qualification remain open. This limitation is retained as evidence rather than
narrowing success to the two readers that passed. No writer superiority claim is
made. Broader unsupported metadata remains outside the documented API.

Raw outputs, source/adapters, comparison traces and PCM reports stay under the
ignored `runs/write-qualification` directory. Reproduction uses the checked-in
`remux_audio` example, pinned reader adapters and bounded local checks
`check_initial.py`, `check_corpus.py` and `check_classic_larger_limit.py` in that
retained directory. External tools/media remain optional research infrastructure;
they are not unlace Cargo dependencies or ordinary test requirements.
