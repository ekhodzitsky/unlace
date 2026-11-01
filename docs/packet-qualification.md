# Selected audio packet qualification

This report covers encoded packet bytes, boundaries, order, selected track and
CodecPrivate on the pinned inputs below. It does not qualify exact timestamps,
decoded samples/trim, seeking, malformed-input recovery, resource performance or
writing. It supports no general compatibility or superiority claim.

## Counterparts and method

All counterpart trials use FFmpeg 7.0.2-static, libavformat 61.1.100, and Mozilla
Nestegg commit `767aab25013acefbdcc6d68a2a7a2c9081303e3f`. Versions were pinned before
comparison. The unlace `audio_packets` example emits selected packets in source
order and requires final source EOF. Nestegg's first audio track is selected by
track type; every packet chunk is emitted separately in its original order.
FFprobe selects `a:0` on the same immutable input. Each input here has one audio
track, so these selections refer to the same track, including mixed video/audio.

Compare each unlace packet's exact bytes with the corresponding Nestegg chunk,
and its SHA-256 with FFprobe's `data_hash`. Require equal counts, boundaries and
order; a missing or extra frame fails. Compare CodecPrivate SHA-256 with FFprobe
stream extradata. The optional local checker processes one manifest line/frame
at a time and refuses absent final completion or data after End. Its ordered
aggregate hash feeds an eight-byte big-endian packet length followed by the
32-byte packet SHA-256 for each packet, to preserve boundaries in the aggregate.

Representative commands, with outputs kept local:

```sh
cargo run --offline --locked --example audio_packets -- INPUT.webm
ffprobe -v error -select_streams a:0 -show_packets -show_data_hash sha256 \
  -show_entries 'packet=data_hash:packet_side_data=' -of 'compact=p=0:nk=1' INPUT.webm
ffprobe -v error -select_streams a:0 -show_streams -show_data_hash sha256 \
  -of json INPUT.webm
```

These verbose correctness manifests are not timed benchmarks. Raw media,
browser profiles, adapters, source metadata and logs remain local and ignored.
Generated browser outputs may differ on a new capture because runtime scheduling
and random track IDs vary. A new capture needs a new hash; do not silently
replace a pinned input or claim a newly generated file has the same identity.

## Real producer corpus

| Input and producer | Input SHA-256 | Selected audio | Packets | Result |
| --- | --- | --- | --- | --- |
| Original 997 Hz oscillator, Chromium 156.0.8078.4 MediaRecorder, audio/webm;codecs=opus | `a299ed51dd7ade5ff37469612f2f8d74f256d075076ee85de3ccfe96cb8a42fc` | Track 1 | 28 | Equal bytes/order with Nestegg, packet hashes and CodecPrivate with FFmpeg |
| Original animated 64x64 canvas plus 997 Hz oscillator, same Chromium, video/webm;codecs=vp9,opus | `362cf539cbeef5b2ed8b297afafd8827a90fca9f4e32ba4a08f8d58a4b45b25e` | Track 2 | 28 | Equal bytes/order with Nestegg, packet hashes and CodecPrivate with FFmpeg |
| Completed YouTube Big Buck Bunny, video ID YE7VzlLtp-4, format 251, Opus audio-only WebM | `0a2e331c1743da8feaca135a2cd5ac967d8d1373d6009318630c68649d750955` | Track 1 | 29,825 | Equal bytes/order with Nestegg, packet hashes and CodecPrivate with FFmpeg |

The browser recordings use Web Audio `createMediaStreamDestination` and native
MediaRecorder, with a 125 ms timeslice; the mixed recording also uses canvas
captureStream at 10 fps. No microphone, camera, personal profile or borrowed
content was used. Concatenated dataavailable chunks are saved unchanged, with no
external remux. Audio-only has 11 actual chunks and mixed has 12, including an
initial one-byte chunk. Replaying both recorded chunk sequences through explicit
WouldBlock continuation yields the same selected packets. Both end with
UnknownSizeSegment, retaining uncertainty about intended producer completeness.

The mixed recording exposed MaxBlockAdditionID=1 and video BlockAdditions.
Regressions preceded the fixes. The reader retains the declaration, checks
unsigned width and singleton occurrence, and skips finite opaque additions only
on unselected tracks after resolving the group's track. Selected additions still
return Unsupported before group packet emission in either metadata order.
Skip size, finite parent boundaries and EOF truncation remain enforced.

The YouTube source is [Big Buck Bunny, uploaded by Blender](https://www.youtube.com/watch?v=YE7VzlLtp-4).
The original film is licensed under Creative Commons Attribution 3.0 by the
[Blender Foundation's Peach project](https://peach.blender.org/about/).
Attribution: (c) copyright 2008, Blender Foundation / www.bigbuckbunny.org.
YouTube's metadata supplied no license field; reuse relies on the original movie
license. The complete 9,730,538-byte format was retrieved by yt-dlp 2026.10.26,
whose asset SHA-256 was verified against its published release digest. Downloader
success and selected-format size were checked; no remux or transcode was applied.

## Limits and unresolved gates

The three earlier generated files also match selected packet hashes with both
counterparts: Opus audio (52 packets), Vorbis live output (21), and VP9/Opus (21).
Those generated files do not substitute for the producer corpus above.

Precise timestamp equivalence is a separate question. FFprobe's millisecond
output quantizes an earlier generated Opus file's first timestamp to -7 ms,
where exact raw 0 ms minus CodecDelay 6.5 ms yields -6.5 ms. No exact timing or
sample-level trim equivalence follows from matching packet bytes. Independent mono/stereo decoding and located timing-metadata checks are now
documented in [opus-timing.md](opus-timing.md), within their own limited scope.

This small corpus does not establish multi-hour behavior, broad browser/YouTube
coverage, sustained fuzzing, seek correctness, memory/allocation measurements or
performance superiority. Those broader requirements remain open.
