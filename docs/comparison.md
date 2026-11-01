# Competitor evidence and selection

Reviewed 2026-11-01 from primary documentation and selected APIs. This review
is supplemented by [pinned source/test/behavior studies](implementation-studies.md)
and [execution measurements](matched-comparison.md), with
separate classic/incremental libwebm variants and explicit metadata gates. Neither
report is a complete security audit. The source pins in that report identify
the tested revisions independently of changing repository branches.
Unlace has an unreleased bounded sequential audio subset; it is not a qualified
demuxer replacement. Its producer, trim and six-hour checks and measured
consumer job have explicit scope; they do not establish the entire target contract.

| Project | Language | Observed capability | Mechanism worth evaluating |
| --- | --- | --- | --- |
| [libwebm](https://github.com/webmproject/libwebm) | C++ | Parser, muxer, separate incremental WebmParser | Feed/status/callback continuation and nonseekable input |
| [Nestegg](https://github.com/mozilla/nestegg/blob/master/include/nestegg/nestegg.h) | C | Packet/track APIs, Cues seek, codec delay, preroll, discard padding | Explicit packet metadata and small demux API |
| [libavformat](https://github.com/FFmpeg/FFmpeg/blob/master/libavformat/matroskadec.c) | C | Matroska/WebM parser with index/seek and padding handling | Independent packet/timestamp/trim comparison and malformed cases |
| [libmatroska](https://github.com/Matroska-Org/libmatroska) | C++ | Low-level Matroska reading/writing over libebml | Schema structure and broader format semantics |
| [GStreamer matroskademux](https://gstreamer.freedesktop.org/documentation/matroska/matroskademux.html) | C | WebM/Matroska stream separation in a pipeline; pull-mode seek settings | Bounded search policy and integration with streamed input |
| [Symphonia MkvReader](https://docs.rs/symphonia/latest/symphonia/default/formats/struct.MkvReader.html) | Rust | Track/packet/seek APIs, chapters and metadata; reviewed docs 0.6.1 | Consumer ergonomics and independent Rust reader |
| [matroska-demuxer](https://github.com/hasenbanck/matroska-demuxer) | Rust | Standalone Read + Seek reader; manifest 0.8.1 has no dependencies; unsafe forbidden | Small API and dependency budget |
| [webm-iterable](https://docs.rs/webm-iterable/latest/webm_iterable/) | Rust | Typed element iteration/writing and an audio-extraction example; reviewed docs 0.7.1 | Separation of EBML mechanics and container schema |
| [Mediabunny](https://github.com/Vanilagy/mediabunny) | TypeScript | Read/write WebM/MKV and other containers, streaming I/O; decoding via WebCodecs | Track selection and convenient streaming media APIs |
| [ebml-go](https://github.com/at-wat/ebml-go) | Go | Marshal/unmarshal over io.Reader/io.Writer; common WebM subset | Reader/writer boundaries and reusable EBML core |

The matroska-demuxer source exposes CodecDelay/SeekPreRoll, but the reviewed
public Frame has no separate DiscardPadding field. The pinned execution
adapter cannot meet the raw trim-field gate. This API limitation is distinct
from a truncation bug. libwebm's incremental parser is distinct from
its older mkvparser API and must be identified correctly in comparisons.

## Adoption and competition

A consumer needing complete demuxing now should evaluate existing libraries.
Pure Rust and zero dependencies alone are not unique differentiators. Unlace's
proposed advantage combines bounded incremental parsing, explicit completeness,
precise audio metadata, verified seek extents and a compact API; the matched report
qualifies a limited selected-packet consumer job, with slower unlace full
iteration and explicit resource/startup tradeoffs. It does not prove overall
superiority or interchangeable validation guarantees.

Study behavior and design choices without assuming that source code may be
relicensed. Record provenance and applicable licenses for any reused code or
fixtures. The initial implementation is independently written from the EBML
integer rules; no competitor implementation was copied.
