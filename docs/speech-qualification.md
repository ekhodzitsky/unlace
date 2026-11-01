# Additional speech corpus qualification

Three short own text-to-speech sources cover English, Russian and German, without
private recordings or external speaker content. Producer: eSpeak NG 1.52.0
(Ubuntu 1.52.0+dfsg-5build1, extracted locally), speed 155 words/minute, the named
language voice, then pinned FFmpeg 7.0.2-static/libopus to mono 48 kHz, 32 kbit/s VBR,
20 ms Opus packets in WebM. Encoding remains an external fixture-production step.
These are synthetic voices, not recognition, intelligibility or natural-speech
codec-quality tests. Original text, WAV, encoded media and exact commands remain
ignored in `runs/objective-audit/speech` and its local scripts.

Every selected source packet byte/order, exact presentation clock, raw signed
padding, CodecPrivate, CodecDelay and SeekPreRoll matches pinned classic and
incremental libwebm plus Nestegg from the [matched report](matched-comparison.md).
Every manifest requires native End. All sources use pre-skip 312 samples, delay
6,500,000 ns and preroll 80,000,000 ns. Whole-file SHA-256 and counts follow.

| Language | Packets | Source SHA-256 | Final padding ns |
| --- | ---: | --- | ---: |
| en | 306 | `f1f871fdcce62826ef9dc7c69d712905a993224dabb66ee086d7644e0e864678` | 3416667 |
| ru | 256 | `9385fae667ec02a28ac7417fa0b7ada409881aac05075ecac60776673fccbee1` | 9270833 |
| de | 286 | `2db6d1dc2f66bf13f8b0533d201e164b31b2ec7705fd6fbd41709732960a46d9` | 16750000 |

The checked-in remux example produces both live and indexed outputs. Unlace
re-reading matches every packet byte/time/padding. The same pinned FFmpeg decoder
produces byte-identical complete mono f32 PCM from each source and both outputs,
including the first and last 64 sample frames. This is scoped decode preservation,
not a new timing benchmark; the original comparison measurements are unchanged.

| Language | PCM frames | Complete PCM SHA-256 | Live output SHA-256 | Indexed output SHA-256 |
| --- | ---: | --- | --- | --- |
| en | 293284 | `70b1ff6f8925c66f70a48974e17347a0e81c395afee67324b13a5a265ad2f4f7` | `cf0f38d8c298749e3b68c7de5c01349733dd574f3fd018a25029a3b04bdcd34d` | `f0492c381905e085932aa44a734908419f257e8e4a335c5f00f98a4625069cd1` |
| ru | 245003 | `88a6eef9c8334aaafce3bb87adadd7bef1eaa911616d3b5ea246224929d692a5` | `1dbccc73b1df2a2496aafc56e1f205d84c903032350252bced4bb0c0b72078c9` | `817d5deb0e4a3b7a30974a1af7891d7e41a8c9e286f86b7faf7918a6454d2a91` |
| de | 273444 | `8dce6efea06d546bffc402a5e525a83ee3a3f1a52d91da11e86aa824118bf6f4` | `c0570f2456f528d2275b7883b75ef132fb5d4a504b4c6f639f5caea6b0f1cc3d` | `93cd3eeca06f04a0a7a538d409ffd618a78c01a4f9b7adccd393e1016e9e2853` |

Reproduction uses `espeak-ng -v LANGUAGE -s 155 -w SOURCE.wav TEXT`, then
`ffmpeg -i SOURCE.wav -ar 48000 -c:a libopus -b:a 32k -frame_duration 20 SOURCE.webm`.
Run the public `audio_packets`/`remux_audio` examples and pinned native adapters,
compare complete manifests, then decode source and both outputs to f32le with the
same decoder. Hashes identify the retained immutable inputs; encoder/container
random metadata means regenerating a source need not reproduce its file hash.
The exact local record is `speech-report.json`; `check_speech.py` requires all
comparisons and rejects missing/extra packets. No media/decoder is needed by
ordinary builds/tests, and no cross-language performance claim is made.
