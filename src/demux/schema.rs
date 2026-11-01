use crate::stream::{ElementSpec, Kind, Parent};

const fn entry(id: u32, parent: u32, kind: Kind) -> ElementSpec {
    ElementSpec {
        id,
        parent: if parent == 0 {
            Parent::Root
        } else {
            Parent::Element(parent)
        },
        kind,
    }
}
const MASTER: Kind = Kind::Master {
    unknown_size: false,
};
const STREAM: Kind = Kind::Master { unknown_size: true };
const META: Kind = Kind::Metadata;
const SKIP: Kind = Kind::Skip;

// Independently transcribed structural IDs from RFC 8794 / RFC 9559.
// Skipped masters are finite and opaque; they never provide seek guarantees.
pub(super) const ENTRIES: &[ElementSpec] = &[
    entry(0x1a45dfa3, 0, MASTER),
    entry(0x4286, 0x1a45dfa3, META),        // EBMLVersion
    entry(0x42f7, 0x1a45dfa3, META),        // EBMLReadVersion
    entry(0x42f2, 0x1a45dfa3, META),        // EBMLMaxIDLength
    entry(0x42f3, 0x1a45dfa3, META),        // EBMLMaxSizeLength
    entry(0x4282, 0x1a45dfa3, META),        // DocType
    entry(0x4287, 0x1a45dfa3, META),        // DocTypeVersion
    entry(0x4285, 0x1a45dfa3, META),        // DocTypeReadVersion
    entry(0x18538067, 0, STREAM),           // Segment
    entry(0x114d9b74, 0x18538067, SKIP),    // SeekHead
    entry(0x1549a966, 0x18538067, MASTER),  // Info
    entry(0x2ad7b1, 0x1549a966, META),      // TimestampScale
    entry(0x4489, 0x1549a966, META),        // Duration
    entry(0x4d80, 0x1549a966, META),        // MuxingApp
    entry(0x5741, 0x1549a966, META),        // WritingApp
    entry(0x73a4, 0x1549a966, SKIP),        // SegmentUUID
    entry(0x4461, 0x1549a966, SKIP),        // DateUTC
    entry(0x7ba9, 0x1549a966, SKIP),        // Title
    entry(0x1654ae6b, 0x18538067, MASTER),  // Tracks
    entry(0xae, 0x1654ae6b, MASTER),        // TrackEntry
    entry(0xd7, 0xae, META),                // TrackNumber
    entry(0x73c5, 0xae, META),              // TrackUID
    entry(0x83, 0xae, META),                // TrackType
    entry(0xb9, 0xae, META),                // FlagEnabled
    entry(0x88, 0xae, META),                // FlagDefault
    entry(0x55aa, 0xae, META),              // FlagForced
    entry(0x9c, 0xae, META),                // FlagLacing
    entry(0x23e383, 0xae, META),            // DefaultDuration
    entry(0x23314f, 0xae, META),            // TrackTimestampScale
    entry(0x537f, 0xae, META),              // TrackOffset
    entry(0x86, 0xae, META),                // CodecID
    entry(0x63a2, 0xae, META),              // CodecPrivate
    entry(0x56aa, 0xae, META),              // CodecDelay
    entry(0x56bb, 0xae, META),              // SeekPreRoll
    entry(0x536e, 0xae, SKIP),              // Name
    entry(0x22b59c, 0xae, SKIP),            // Language
    entry(0x22b59d, 0xae, SKIP),            // LanguageBCP47
    entry(0x258688, 0xae, SKIP),            // CodecName
    entry(0x6d80, 0xae, META),              // ContentEncodings: explicitly unsupported
    entry(0x41e4, 0xae, META),              // BlockAdditionMapping: unsupported
    entry(0xe2, 0xae, META),                // TrackOperation: unsupported
    entry(0xe0, 0xae, SKIP),                // Video: nonselected track details are opaque
    entry(0xe1, 0xae, MASTER),              // Audio
    entry(0xb5, 0xe1, META),                // SamplingFrequency
    entry(0x78b5, 0xe1, META),              // OutputSamplingFrequency
    entry(0x9f, 0xe1, META),                // Channels
    entry(0x6264, 0xe1, META),              // BitDepth
    entry(0x1f43b675, 0x18538067, STREAM),  // Cluster
    entry(0xe7, 0x1f43b675, META),          // Timestamp
    entry(0xa7, 0x1f43b675, META),          // Position
    entry(0xab, 0x1f43b675, META),          // PrevSize
    entry(0xa3, 0x1f43b675, Kind::Payload), // SimpleBlock
    entry(0xa0, 0x1f43b675, MASTER),        // BlockGroup
    entry(0xa1, 0xa0, Kind::Payload),       // Block
    entry(0x9b, 0xa0, META),                // BlockDuration
    entry(0xfb, 0xa0, META),                // ReferenceBlock (repeatable)
    entry(0x75a2, 0xa0, META),              // DiscardPadding
    entry(0xfa, 0xa0, META),                // ReferencePriority
    entry(0xa4, 0xa0, META),                // CodecState: unsupported
    entry(0x75a1, 0xa0, META),              // BlockAdditions: unsupported
    entry(0x1c53bb6b, 0x18538067, SKIP),    // Cues
    entry(0x1254c367, 0x18538067, SKIP),    // Tags
    entry(0x1043a770, 0x18538067, SKIP),    // Chapters
    entry(0x1941a469, 0x18538067, SKIP),    // Attachments
    ElementSpec {
        id: 0xec,
        parent: Parent::Global,
        kind: SKIP,
    }, // Void
    ElementSpec {
        id: 0xbf,
        parent: Parent::Global,
        kind: META,
    }, // CRC: unsupported
];

pub(super) fn repeatable(id: u32) -> bool {
    matches!(
        id,
        0xae | 0x1f43b675 | 0xa3 | 0xa0 | 0xfb | 0x114d9b74 | 0xec
    )
}
