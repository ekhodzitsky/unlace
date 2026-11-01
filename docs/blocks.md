# Individual encoded block parsing

The unreleased `block::parse_block` function splits the complete payload of one
Matroska Block or SimpleBlock, following [RFC 9559 sections 10.1 through 10.3](https://www.rfc-editor.org/rfc/rfc9559.html#section-10).
It does not read a WebM file or select an audio track. The caller must first
obtain exactly one complete finite element payload, enforce the stream's declared
size, and supply the corresponding `BlockKind`. A short slice after the block
header may itself describe a valid smaller frame; only the framing layer can
detect that the original source was truncated. Do not use this parser as a source
completeness check.

The result retains the nonzero track number, original flags, signed relative
timestamp and original lacing mode. `frames()` returns exact borrowed frame
slices in their encoded order. No frame data is copied or allocated. All one-
through-eight-byte track VINT encodings are supported. VINT data containing all
ones is a value here; it is not the element-size unknown sentinel. Reserved flag
bits are rejected separately for Block and SimpleBlock. BlockGroup keyframe
status requires ReferenceBlock metadata, so Block keyframe/discardable accessors
return None. The unused low Block flag bit is preserved without interpretation.

No lacing yields one frame. Fixed lacing requires evenly divisible data. Xiph
lacing checks continuation sums, and EBML lacing checks unsigned first size and
signed differences with the width-dependent bias. Sizes must partition the
remaining payload exactly, without negative lengths, offset overflow or overruns.
The final size in Xiph/EBML mode is derived from the remaining bytes. Lacing a
single frame is invalid. Empty encoded frames remain visible: their codec
validity cannot be decided by structural parsing.

The signed relative timestamp applies to the first frame and is measured in
Track Ticks. Later laced-frame timestamps require duration or codec information
and are underdetermined by the block itself. This API does not assign duplicate
timestamps, apply TimestampScale/TrackTimestampScale, subtract CodecDelay or
apply DiscardPadding. Those operations belong to a track-aware container layer.
Track FlagLacing and other document-level constraints also remain that layer's
responsibility. Passing this parser is not evidence that the track is supported,
unencrypted or free of content transformations.

`BlockLimits` bounds complete block bytes, individual frame bytes, frame count
and lace header bytes. Defaults are 16 MiB for block/frame bytes, 256 frames and
64 KiB of lace header, including the count byte. The block size is checked before
parsing. Xiph continuation work is bounded even when every size byte is 255.
The result uses fixed storage for 256 lengths, regardless of recording duration;
caller-owned input storage must separately respect the block limit. No entire
recording needs to be buffered. See the executable rustdoc example in the module.
