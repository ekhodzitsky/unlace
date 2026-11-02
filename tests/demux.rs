//! Track-aware sequential demux regressions using original synthetic containers.
#![allow(clippy::unwrap_used)]
use std::io::{self, Read};
use unlace::{demux::*, stream::StreamError};

const EBML: u32 = 0x1a45dfa3;
const SEGMENT: u32 = 0x18538067;
const INFO: u32 = 0x1549a966;
const TRACKS: u32 = 0x1654ae6b;
const CLUSTER: u32 = 0x1f43b675;

fn size(value: usize) -> Vec<u8> {
    let width = (1..=8)
        .find(|width| (value as u64) < (1u64 << (7 * width)) - 1)
        .unwrap();
    let encoded = value as u64 | (1u64 << (7 * width));
    encoded.to_be_bytes()[8 - width..].to_vec()
}
fn element(id: u32, data: &[u8]) -> Vec<u8> {
    let bytes = id.to_be_bytes();
    let first = bytes.iter().position(|&b| b != 0).unwrap();
    let mut result = bytes[first..].to_vec();
    result.extend(size(data.len()));
    result.extend(data);
    result
}
fn unknown(id: u32, data: &[u8]) -> Vec<u8> {
    let bytes = id.to_be_bytes();
    let first = bytes.iter().position(|&b| b != 0).unwrap();
    let mut result = bytes[first..].to_vec();
    result.push(255);
    result.extend(data);
    result
}
fn number(id: u32, value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    element(id, &bytes[first..])
}
fn header(extra: &[u8]) -> Vec<u8> {
    let mut data = element(0x4282, b"webm");
    data.extend(number(0x4287, 4));
    data.extend(number(0x4285, 2));
    data.extend(extra);
    element(EBML, &data)
}
fn info(extra: &[u8]) -> Vec<u8> {
    let mut data = element(0x4d80, b"unlace test");
    data.extend(element(0x5741, b"independent synthetic writer"));
    data.extend(extra);
    element(INFO, &data)
}
fn track(number_: u64, audio: bool, extra: &[u8]) -> Vec<u8> {
    let mut data = number(0xd7, number_);
    data.extend(number(0x73c5, 100 + number_));
    data.extend(number(0x83, if audio { 2 } else { 1 }));
    data.extend(element(0x86, if audio { b"A_TEST" } else { b"V_VP9" }));
    if audio {
        data.extend(element(0x63a2, &[0, 1, 2]));
        let mut settings = element(0xb5, &48000f64.to_be_bytes());
        settings.extend(number(0x9f, 2));
        data.extend(element(0xe1, &settings));
    } else {
        data.extend(element(0xe0, &[0x00, 0xff, 0x00]));
    } // opaque, unselected video metadata
    data.extend(extra);
    element(0xae, &data)
}
fn block(track: u8, timestamp: i16, flags: u8, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x80 | track];
    bytes.extend(timestamp.to_be_bytes());
    bytes.push(flags);
    bytes.extend(payload);
    bytes
}
fn document(
    track_data: &[u8],
    clusters: &[u8],
    info_extra: &[u8],
    header_extra: &[u8],
    unknown_segment: bool,
) -> Vec<u8> {
    let mut data = info(info_extra);
    data.extend(element(TRACKS, track_data));
    data.extend(clusters);
    let mut result = header(header_extra);
    result.extend(if unknown_segment {
        unknown(SEGMENT, &data)
    } else {
        element(SEGMENT, &data)
    });
    result
}
fn fixture(unknown_sizes: bool) -> Vec<u8> {
    let mut tracks = track(1, false, &[]);
    let mut extras = number(0x56aa, 7);
    extras.extend(number(0x56bb, 80_000_000));
    extras.extend(number(0x23e383, 20_000_000));
    tracks.extend(track(2, true, &extras));
    let mut cluster = number(0xe7, 100);
    cluster.extend(element(0xa3, &block(1, 0, 0x80, &[90, 91])));
    cluster.extend(element(0xa3, &block(2, -2, 0x80, &[1, 2, 3])));
    let mut group = element(0xa1, &block(2, 1, 4, &[1, 4, 5, 6, 7])); // two fixed-laced frames
    group.extend(number(0x9b, 40));
    group.extend(element(0xfb, &[0xff]));
    group.extend(element(0x75a2, &[9]));
    cluster.extend(element(0xa0, &group));
    let clusters = if unknown_sizes {
        unknown(CLUSTER, &cluster)
    } else {
        element(CLUSTER, &cluster)
    };
    document(&tracks, &clusters, &[], &[], unknown_sizes)
}

#[derive(Debug, PartialEq, Eq)]
struct Output {
    track: u64,
    data: Vec<u8>,
    timestamp: Option<i128>,
    origin: i128,
    index: u16,
    count: u16,
    padding: i64,
    duration: Option<i128>,
    keyframe: bool,
    offset: u64,
}
fn drain<R: Read>(demux: &mut Demuxer<R>) -> Result<(Vec<Output>, Completion), DemuxError> {
    let mut output = Vec::new();
    for _ in 0..100_000 {
        match demux.poll()? {
            DemuxEvent::Packet(packet) => output.push(Output {
                track: packet.track_number,
                data: packet.data.to_vec(),
                timestamp: packet.timestamp_ns,
                origin: packet.block_timestamp_ns,
                index: packet.lace_index,
                count: packet.lace_count,
                padding: packet.discard_padding_ns,
                duration: packet.block_duration_ns,
                keyframe: packet.keyframe,
                offset: packet.block_offset,
            }),
            DemuxEvent::End(completion) => return Ok((output, completion)),
            _ => (),
        }
    }
    panic!("demux failed to make bounded progress")
}
fn reader(bytes: &[u8]) -> Demuxer<&[u8]> {
    Demuxer::new(bytes, AudioSelection::First, DemuxLimits::default()).unwrap()
}

#[test]
fn selected_audio_packets_have_exact_order_tracks_timestamps_and_late_group_metadata() {
    for unknown_sizes in [false, true] {
        let data = fixture(unknown_sizes);
        let mut demux = reader(&data);
        let (packets, completion) = drain(&mut demux).unwrap();
        assert_eq!(
            completion,
            if unknown_sizes {
                Completion::UnknownSizeSegment
            } else {
                Completion::FiniteSegment
            }
        );
        assert_eq!(
            packets
                .iter()
                .map(|p| p.data.as_slice())
                .collect::<Vec<_>>(),
            [&[1, 2, 3][..], &[4, 5], &[6, 7]]
        );
        assert_eq!(
            packets.iter().map(|p| p.track).collect::<Vec<_>>(),
            [2, 2, 2]
        );
        assert_eq!(
            packets.iter().map(|p| p.timestamp).collect::<Vec<_>>(),
            [Some(97_999_993), Some(100_999_993), Some(120_999_993)]
        );
        assert_eq!(
            packets.iter().map(|p| p.padding).collect::<Vec<_>>(),
            [0, 0, 9]
        );
        assert_eq!(
            packets.iter().map(|p| p.keyframe).collect::<Vec<_>>(),
            [true, false, false]
        );
        assert_eq!(packets[1].duration, Some(40_000_000));
        assert_eq!(packets[1].offset, packets[2].offset);
        for packet in &packets {
            assert!(matches!(data[packet.offset as usize], 0xa1 | 0xa3));
        }
        assert_eq!(demux.tracks().len(), 2);
        let audio = &demux.tracks()[1];
        assert_eq!(audio.codec_id, "A_TEST");
        assert_eq!(audio.codec_private, [0, 1, 2]);
        assert_eq!(audio.seek_preroll_ns, 80_000_000);
        assert_eq!(audio.audio.unwrap().sampling_frequency, 48000.0);
        assert_eq!(audio.audio.unwrap().channels, 2);
        assert_eq!(demux.position(), data.len() as u64);
        assert!(matches!(demux.poll().unwrap(), DemuxEvent::End(value) if value == completion));
    }
}

#[test]
fn empty_elements_with_declared_defaults_use_those_defaults() {
    let mut entry = number(0xd7, 1);
    entry.extend(number(0x73c5, 1));
    entry.extend(number(0x83, 2));
    entry.extend(element(0x86, b"A_TEST"));
    entry.extend(element(0xb9, &[]));
    entry.extend(element(0x9c, &[]));
    entry.extend(element(0x23314f, &[]));
    let mut audio = element(0xb5, &[]);
    audio.extend(element(0x9f, &[]));
    audio.extend(element(0x78b5, &[]));
    entry.extend(element(0xe1, &audio));
    let mut declarations = element(0x4286, &[]);
    declarations.extend(element(0x42f7, &[]));
    declarations.extend(element(0x42f2, &[]));
    declarations.extend(element(0x42f3, &[]));
    let data = document(
        &element(0xae, &entry),
        &element(CLUSTER, &number(0xe7, 0)),
        &element(0x2ad7b1, &[]),
        &declarations,
        false,
    );
    let mut demux = reader(&data);
    drain(&mut demux).unwrap();
    assert_eq!(demux.info().unwrap().timestamp_scale_ns, 1_000_000);
    let track = &demux.tracks()[0];
    assert!(track.enabled && track.lacing_allowed);
    assert_eq!(track.audio.unwrap().sampling_frequency, 8000.0);
    assert_eq!(track.audio.unwrap().output_sampling_frequency, 8000.0);
    assert_eq!(track.audio.unwrap().channels, 1);
}

struct Source<'a> {
    bytes: &'a [u8],
    position: usize,
    chunk: usize,
    fail: Option<(usize, io::ErrorKind)>,
    calls: usize,
}
impl Read for Source<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;
        if self
            .fail
            .is_some_and(|(position, _)| position == self.position)
        {
            let (_, kind) = self.fail.take().unwrap();
            return Err(io::Error::new(kind, "injected source error"));
        }
        let count = output
            .len()
            .min(self.chunk)
            .min(self.bytes.len() - self.position);
        output[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}
fn source(bytes: &[u8], chunk: usize) -> Source<'_> {
    Source {
        bytes,
        position: 0,
        chunk,
        fail: None,
        calls: 0,
    }
}

#[test]
fn every_byte_continuation_interruption_and_source_failure_is_exact() {
    let data = fixture(false);
    let expected = drain(&mut reader(&data)).unwrap();
    for chunk in [1, 2, 3, 7, 16] {
        let mut demux = Demuxer::new(
            source(&data, chunk),
            AudioSelection::First,
            DemuxLimits::default(),
        )
        .unwrap();
        assert_eq!(drain(&mut demux).unwrap(), expected);
    }
    for position in 0..=data.len() {
        for kind in [io::ErrorKind::WouldBlock, io::ErrorKind::Interrupted] {
            let mut input = source(&data, 1);
            input.fail = Some((position, kind));
            let mut demux =
                Demuxer::new(input, AudioSelection::First, DemuxLimits::default()).unwrap();
            assert_eq!(drain(&mut demux).unwrap(), expected, "{position} {kind:?}");
        }
        let mut input = source(&data, 1);
        input.fail = Some((position, io::ErrorKind::PermissionDenied));
        let mut demux = Demuxer::new(input, AudioSelection::First, DemuxLimits::default()).unwrap();
        assert!(
            matches!(drain(&mut demux), Err(DemuxError::Stream(StreamError::Io(error))) if error.kind() == io::ErrorKind::PermissionDenied)
        );
        assert!(matches!(demux.poll(), Err(DemuxError::Failed)));
    }
}

#[test]
fn every_finite_fixture_truncation_fails_including_after_earlier_packets() {
    let data = fixture(false);
    for length in 0..data.len() {
        let mut demux = reader(&data[..length]);
        assert!(drain(&mut demux).is_err(), "cut {length}");
        assert!(matches!(demux.poll(), Err(DemuxError::Failed)));
    }
    assert!(matches!(
        drain(&mut reader(&data[..data.len() - 1])),
        Err(DemuxError::Stream(StreamError::Truncated { .. }))
    ));
}

#[test]
fn all_container_laces_keep_original_frames_and_expose_underdetermined_timestamps() {
    let tracks = track(1, true, &[]);
    let mut cluster = number(0xe7, 5);
    let descriptors: &[(u8, &[u8])] = &[
        (0, &[1, 2]),
        (2, &[1, 2, 1, 2, 3, 4]), // 2 frames, first len 2
        (4, &[1, 1, 2, 3, 4]),
        (6, &[1, 0x82, 1, 2, 3, 4]),
    ];
    for &(flags, payload) in descriptors {
        cluster.extend(element(0xa3, &block(1, -6, flags, payload)));
    }
    let data = document(&tracks, &element(CLUSTER, &cluster), &[], &[], false);
    let packets = drain(&mut reader(&data)).unwrap().0;
    assert_eq!(packets.len(), 7);
    for (index, packet) in packets.iter().enumerate() {
        assert_eq!(
            packet.data,
            if index == 0 || index % 2 == 1 {
                vec![1, 2]
            } else {
                vec![3, 4]
            }
        );
        assert_eq!(
            packet.timestamp,
            if index == 0 || index % 2 == 1 {
                Some(-1_000_000)
            } else {
                None
            }
        );
        assert_eq!(packet.origin, -1_000_000);
    }
}

#[test]
fn negative_padding_attaches_only_to_first_frame_without_removing_encoded_bytes() {
    let mut group = element(0x75a2, &[0xff]); // -1 ns, before Block
    group.extend(element(0xa1, &block(1, 0, 4, &[1, 8, 9])));
    let mut cluster = number(0xe7, 0);
    cluster.extend(element(0xa0, &group));
    let data = document(
        &track(1, true, &[]),
        &element(CLUSTER, &cluster),
        &[],
        &[],
        false,
    );
    let packets = drain(&mut reader(&data)).unwrap().0;
    assert_eq!(
        packets.iter().map(|p| p.padding).collect::<Vec<_>>(),
        [-1, 0]
    );
    assert_eq!(
        packets
            .iter()
            .map(|p| p.data.as_slice())
            .collect::<Vec<_>>(),
        [&[8][..], &[9]]
    );
    assert!(packets.iter().all(|p| p.keyframe));
}

#[test]
fn explicit_audio_selection_uses_stable_track_number_and_enabled_flag() {
    let mut tracks = track(4, false, &[]);
    tracks.extend(track(9, true, &number(0xb9, 0)));
    tracks.extend(track(3, true, &[]));
    tracks.extend(track(12, true, &[]));
    let mut cluster = number(0xe7, 0);
    for number in [4, 9, 3, 12] {
        cluster.extend(element(0xa3, &block(number, 0, 0, &[number])));
    }
    let data = document(&tracks, &element(CLUSTER, &cluster), &[], &[], false);
    assert_eq!(
        drain(&mut reader(&data))
            .unwrap()
            .0
            .iter()
            .map(|p| p.track)
            .collect::<Vec<_>>(),
        [3]
    );
    let mut demux = Demuxer::new(
        data.as_slice(),
        AudioSelection::Number(12),
        DemuxLimits::default(),
    )
    .unwrap();
    assert_eq!(
        drain(&mut demux)
            .unwrap()
            .0
            .iter()
            .map(|p| p.track)
            .collect::<Vec<_>>(),
        [12]
    );
    for number in [4, 9, 99] {
        let mut demux = Demuxer::new(
            data.as_slice(),
            AudioSelection::Number(number),
            DemuxLimits::default(),
        )
        .unwrap();
        assert!(matches!(
            drain(&mut demux),
            Err(DemuxError::AudioTrackUnavailable)
        ));
    }
}

#[test]
fn unsupported_transformations_encryption_codec_state_and_crc_are_explicit() {
    for id in [0x6d80, 0x41e4, 0xe2] {
        // Refused even on an unselected video track, including empty masters.
        let mut tracks = track(1, false, &element(id, &[]));
        tracks.extend(track(2, true, &[]));
        let data = document(&tracks, &[], &[], &[], false);
        assert!(matches!(
            drain(&mut reader(&data)),
            Err(DemuxError::Unsupported(_))
        ));
    }
    for id in [0xa4, 0x75a1] {
        let mut group = element(0xa1, &block(1, 0, 0, &[1]));
        group.extend(element(id, &[]));
        let mut cluster = number(0xe7, 0);
        cluster.extend(element(0xa0, &group));
        let data = document(
            &track(1, true, &[]),
            &element(CLUSTER, &cluster),
            &[],
            &[],
            false,
        );
        let mut demux = reader(&data);
        let mut packets = 0;
        loop {
            match demux.poll() {
                Ok(DemuxEvent::Packet(_)) => packets += 1,
                Err(DemuxError::Unsupported(_)) => break,
                Err(error) => panic!("{error}"),
                _ => (),
            }
        }
        assert_eq!(packets, 0); // BlockGroup cannot leak a packet before late fields.
    }
    let data = document(
        &track(1, true, &[]),
        &[],
        &element(0xbf, &[0; 4]),
        &[],
        false,
    );
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Unsupported("CRC-32 verification"))
    ));
    let data = document(
        &track(1, true, &element(0x23314f, &2f64.to_be_bytes())),
        &[],
        &[],
        &[],
        false,
    );
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Unsupported("nonunit TrackTimestampScale"))
    ));
}

#[test]
fn singleton_duplicates_missing_fields_unknown_tracks_and_bad_lacing_are_invalid() {
    for tracks in [
        [track(1, true, &[]), track(1, true, &[])].concat(),
        track(1, true, &number(0xd7, 2)),
        element(0xae, &number(0xd7, 1)),
    ] {
        let data = document(&tracks, &[], &[], &[], false);
        assert!(matches!(
            drain(&mut reader(&data)),
            Err(DemuxError::Invalid(_))
        ));
    }
    for (track_extra, block_data) in [
        (&[][..], block(2, 0, 0, &[1])),
        (&number(0x9c, 0)[..], block(1, 0, 4, &[1, 1, 2])),
        (&[][..], block(1, 0, 4, &[0, 1])),
    ] {
        let mut cluster = number(0xe7, 0);
        cluster.extend(element(0xa3, &block_data));
        let data = document(
            &track(1, true, track_extra),
            &element(CLUSTER, &cluster),
            &[],
            &[],
            false,
        );
        assert!(drain(&mut reader(&data)).is_err());
    }
    let mut data = header(&[]);
    data.extend(element(SEGMENT, &element(TRACKS, &track(1, true, &[]))));
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Invalid(_))
    ));
    let mut cluster = number(0xe7, 0);
    cluster.extend(number(0xe7, 0));
    let data = document(
        &track(1, true, &[]),
        &element(CLUSTER, &cluster),
        &[],
        &[],
        false,
    );
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Timing(TimingError {
            kind: TimingErrorKind::DuplicateElement,
            element_id: 0xe7,
            ..
        }))
    ));
}

#[test]
fn declared_width_versions_scalar_lengths_and_parent_boundaries_are_enforced() {
    for extra in [number(0x42f2, 3), number(0x42f3, 0), number(0x42f3, 9)] {
        let data = document(&track(1, true, &[]), &[], &[], &extra, false);
        assert!(matches!(
            drain(&mut reader(&data)),
            Err(DemuxError::Invalid(_))
        ));
    }
    let mut data = header(&number(0x42f3, 1));
    let mut body = info(&[]);
    body.extend(element(TRACKS, &track(1, true, &[])));
    data.extend(SEGMENT.to_be_bytes());
    data.extend([0x40, 0]);
    data.extend(body);
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Stream(StreamError::Limit("size width")))
    ));
    for (extra, expected) in [
        ([number(0x4286, 2), number(0x42f7, 2)].concat(), "version"),
        (number(0x4286, 2), "version"),
    ] {
        let data = document(&track(1, true, &[]), &[], &[], &extra, false);
        assert!(
            matches!(drain(&mut reader(&data)), Err(DemuxError::Unsupported(_))),
            "{expected}"
        );
    }
    for extra in [
        element(0x2ad7b1, &[0; 9]),
        element(0x4489, &[0; 3]),
        element(0x4489, &f64::NAN.to_be_bytes()),
        number(0x2ad7b1, 0),
    ] {
        let data = document(&track(1, true, &[]), &[], &extra, &[], false);
        assert!(matches!(
            drain(&mut reader(&data)),
            Err(DemuxError::Timing(TimingError {
                kind: TimingErrorKind::InvalidScalar,
                ..
            }))
        ));
    }
    let mut data = header(&[]);
    data.extend(SEGMENT.to_be_bytes());
    data.extend([0x81, 0x15]);
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Stream(StreamError::Invalid(_)))
    ));
}

#[test]
fn aggregate_metadata_track_block_and_work_limits_bound_processing() {
    let data = fixture(false);
    for (limits, reason) in [
        (
            DemuxLimits {
                max_metadata_size: 1,
                ..DemuxLimits::default()
            },
            "metadata size",
        ),
        (
            DemuxLimits {
                max_tracks: 1,
                ..DemuxLimits::default()
            },
            "track count",
        ),
        (
            DemuxLimits {
                block: unlace::block::BlockLimits {
                    max_block_size: 1,
                    ..Default::default()
                },
                ..DemuxLimits::default()
            },
            "block size",
        ),
    ] {
        let mut demux = Demuxer::new(data.as_slice(), AudioSelection::First, limits).unwrap();
        assert!(matches!(drain(&mut demux), Err(DemuxError::Limit(actual)) if actual == reason));
    }
    let limits = DemuxLimits {
        max_events_per_poll: 1,
        framing: unlace::stream::Limits {
            max_read_bytes: 1,
            max_read_calls: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut demux = Demuxer::new(source(&data, 1), AudioSelection::First, limits).unwrap();
    let mut packets = Vec::new();
    loop {
        let before = demux.position();
        let ended = match demux.poll().unwrap() {
            DemuxEvent::End(_) => true,
            DemuxEvent::Packet(p) => {
                packets.push(p.data.to_vec());
                false
            }
            _ => false,
        };
        assert!(demux.position() - before <= 1);
        if ended {
            break;
        }
    }
    assert_eq!(packets, [vec![1, 2, 3], vec![4, 5], vec![6, 7]]);
    for limits in [
        DemuxLimits {
            max_tracks: 0,
            ..Default::default()
        },
        DemuxLimits {
            max_events_per_poll: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            Demuxer::new(&[][..], AudioSelection::First, limits),
            Err(DemuxError::Configuration)
        ));
    }
}

#[test]
fn timestamp_arithmetic_retains_large_values_and_refuses_overflow() {
    let mut cluster = number(0xe7, u64::MAX);
    cluster.extend(element(0xa3, &block(1, i16::MAX, 0, &[1])));
    let data = document(
        &track(1, true, &[]),
        &element(CLUSTER, &cluster),
        &number(0x2ad7b1, 1),
        &[],
        false,
    );
    assert_eq!(
        drain(&mut reader(&data)).unwrap().0[0].timestamp,
        Some(i128::from(u64::MAX) + i128::from(i16::MAX))
    );
    let data = document(
        &track(1, true, &[]),
        &element(CLUSTER, &cluster),
        &number(0x2ad7b1, u64::MAX),
        &[],
        false,
    );
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Timing(TimingError {
            kind: TimingErrorKind::ArithmeticOverflow,
            ..
        }))
    ));
}

#[test]
fn successive_unknown_clusters_and_no_indexes_remain_streamable() {
    let mut clusters = Vec::new();
    for timestamp in 0..10_000 {
        let mut cluster = number(0xe7, timestamp);
        cluster.extend(element(0xa3, &block(1, 0, 0, &[7])));
        clusters.extend(unknown(CLUSTER, &cluster));
    }
    let data = document(&track(1, true, &[]), &clusters, &[], &[], true);
    let (packets, completion) = drain(&mut reader(&data)).unwrap();
    assert_eq!(packets.len(), 10_000);
    assert_eq!(packets.last().unwrap().timestamp, Some(9_999_000_000));
    assert_eq!(completion, Completion::UnknownSizeSegment);
}

#[test]
fn zero_duration_is_outside_the_documented_positive_range() {
    let data = document(
        &track(1, true, &[]),
        &[],
        &element(0x4489, &0f64.to_be_bytes()),
        &[],
        false,
    );
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Timing(TimingError {
            kind: TimingErrorKind::InvalidScalar,
            element_id: 0x4489,
            ..
        }))
    ));
}

#[test]
fn blockgroup_truncation_never_exposes_its_block_before_group_end() {
    let mut group = element(0xa1, &block(1, 0, 0, &[8, 9]));
    group.extend(element(0x75a2, &[0, 1]));
    let mut cluster = number(0xe7, 0);
    cluster.extend(element(0xa0, &group));
    let data = document(
        &track(1, true, &[]),
        &element(CLUSTER, &cluster),
        &[],
        &[],
        false,
    );
    let mut demux = reader(&data[..data.len() - 1]);
    loop {
        match demux.poll() {
            Ok(DemuxEvent::Packet(_)) => {
                panic!("BlockGroup leaked a packet before metadata was complete")
            }
            Ok(DemuxEvent::End(_)) => panic!("truncated BlockGroup succeeded"),
            Err(DemuxError::Stream(StreamError::Truncated { .. })) => break,
            Err(error) => panic!("unexpected failure: {error}"),
            _ => (),
        }
    }
}

#[test]
fn unsupported_order_and_chained_documents_do_not_look_like_clean_completion() {
    let tracks = element(TRACKS, &track(1, true, &[]));
    let mut data = header(&[]);
    let mut body = element(CLUSTER, &number(0xe7, 0));
    body.extend(info(&[]));
    body.extend(tracks);
    data.extend(element(SEGMENT, &body));
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Unsupported(_))
    ));
    let mut data = fixture(true);
    data.extend(fixture(false));
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Unsupported("chained EBML documents"))
    ));
    let mut data = fixture(false);
    data.extend(element(0xec, &[0, 0]));
    assert_eq!(
        drain(&mut reader(&data)).unwrap().1,
        Completion::FiniteSegment
    );
}

#[test]
fn browser_max_block_addition_id_declaration_does_not_block_other_track_audio() {
    let mut tracks = track(1, false, &number(0x55ee, 1));
    tracks.extend(track(2, true, &[]));
    let mut cluster = number(0xe7, 0);
    cluster.extend(element(0xa3, &block(1, 0, 0x80, &[9])));
    cluster.extend(element(0xa3, &block(2, 0, 0x80, &[1, 2])));
    let data = document(&tracks, &element(CLUSTER, &cluster), &[], &[], false);
    let mut demux = reader(&data);
    let (packets, completion) = drain(&mut demux).unwrap();
    assert_eq!(demux.tracks()[0].max_block_addition_id, 1);
    assert_eq!(demux.tracks()[1].max_block_addition_id, 0);
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].data, [1, 2]);
    assert_eq!(packets[0].track, 2);
    assert_eq!(completion, Completion::FiniteSegment);
}

#[test]
fn max_block_addition_id_checks_width_and_singleton_occurrence() {
    for extra in [
        element(0x55ee, &[0; 9]),
        [number(0x55ee, 1), number(0x55ee, 1)].concat(),
    ] {
        let data = document(&track(1, true, &extra), &[], &[], &[], false);
        assert!(matches!(
            drain(&mut reader(&data)),
            Err(DemuxError::Invalid(_))
        ));
    }
    for value in [0, 1, u64::MAX] {
        let data = document(
            &track(1, true, &number(0x55ee, value)),
            &[],
            &[],
            &[],
            false,
        );
        let mut demux = reader(&data);
        drain(&mut demux).unwrap();
        assert_eq!(demux.tracks()[0].max_block_addition_id, value);
    }
    let data = document(&track(1, true, &element(0x55ee, &[])), &[], &[], &[], false);
    let mut demux = reader(&data);
    drain(&mut demux).unwrap();
    assert_eq!(demux.tracks()[0].max_block_addition_id, 0);
}

#[test]
fn audio_selection_skips_bounded_unselected_video_additions_in_either_order() {
    for additions_first in [false, true] {
        let mut tracks = track(1, false, &number(0x55ee, 1));
        tracks.extend(track(2, true, &[]));
        let addition = element(0x75a1, &element(0xa6, &element(0xa5, &[3, 4])));
        let video_block = element(0xa1, &block(1, 0, 0, &[9]));
        let group = if additions_first {
            [addition, video_block].concat()
        } else {
            [video_block, addition].concat()
        };
        let mut cluster = number(0xe7, 0);
        cluster.extend(element(0xa0, &group));
        cluster.extend(element(0xa3, &block(2, 0, 0x80, &[1, 2])));
        let data = document(&tracks, &element(CLUSTER, &cluster), &[], &[], false);
        let packets = drain(&mut reader(&data)).unwrap().0;
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].data, [1, 2]);
        assert_eq!(packets[0].track, 2);
    }
}

#[test]
fn selected_track_additions_are_rejected_before_any_group_packet_in_either_order() {
    for additions_first in [false, true] {
        let addition = element(0x75a1, &element(0xa6, &element(0xa5, &[3, 4])));
        let audio_block = element(0xa1, &block(1, 0, 0, &[1, 2]));
        let group = if additions_first {
            [addition, audio_block].concat()
        } else {
            [audio_block, addition].concat()
        };
        let mut cluster = number(0xe7, 0);
        cluster.extend(element(0xa0, &group));
        let data = document(
            &track(1, true, &number(0x55ee, 1)),
            &element(CLUSTER, &cluster),
            &[],
            &[],
            false,
        );
        let mut demux = reader(&data);
        loop {
            match demux.poll() {
                Ok(DemuxEvent::Packet(_)) => {
                    panic!("selected-track additions were silently discarded")
                }
                Err(DemuxError::Unsupported("block additions")) => break,
                Err(error) => panic!("{error}"),
                Ok(DemuxEvent::End(_)) => panic!("selected-track additions accepted"),
                _ => (),
            }
        }
    }
}

#[test]
fn ignored_additions_still_enforce_skip_limits_boundaries_and_final_eof() {
    let mut tracks = track(1, false, &number(0x55ee, 1));
    tracks.extend(track(2, true, &[]));
    let mut group = element(0xa1, &block(1, 0, 0, &[9]));
    group.extend(element(0x75a1, &element(0xa6, &element(0xa5, &[3, 4, 5]))));
    let mut cluster = number(0xe7, 0);
    cluster.extend(element(0xa0, &group));
    let data = document(&tracks, &element(CLUSTER, &cluster), &[], &[], false);
    let limits = DemuxLimits {
        framing: unlace::stream::Limits {
            max_skip_size: 3,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut demux = Demuxer::new(data.as_slice(), AudioSelection::First, limits).unwrap();
    assert!(matches!(
        drain(&mut demux),
        Err(DemuxError::Stream(StreamError::Limit("skip size")))
    ));
    assert!(matches!(
        drain(&mut reader(&data[..data.len() - 1])),
        Err(DemuxError::Stream(StreamError::Truncated { .. }))
    ));
    // The additions header declares a byte beyond its enclosing finite group.
    let mut group = element(0xa1, &block(1, 0, 0, &[9]));
    group.extend([0x75, 0xa1, 0x81]);
    let mut cluster = number(0xe7, 0);
    cluster.extend(element(0xa0, &group));
    let data = document(&tracks, &element(CLUSTER, &cluster), &[], &[], false);
    assert!(matches!(
        drain(&mut reader(&data)),
        Err(DemuxError::Stream(StreamError::Invalid(_)))
    ));
}

fn opus_track(private: &[u8], delay: Option<u64>, channels: u64) -> Vec<u8> {
    let mut entry = number(0xd7, 1);
    entry.extend(number(0x73c5, 1));
    entry.extend(number(0x83, 2));
    entry.extend(element(0x86, b"A_OPUS"));
    entry.extend(element(0x63a2, private));
    if let Some(delay) = delay {
        entry.extend(number(0x56aa, delay));
    }
    let mut audio = element(0xb5, &48000f64.to_be_bytes());
    audio.extend(number(0x9f, channels));
    entry.extend(element(0xe1, &audio));
    element(0xae, &entry)
}
fn opus_head(pre_skip: u16) -> Vec<u8> {
    let mut head = b"OpusHead".to_vec();
    head.extend([1, 2]);
    head.extend(pre_skip.to_le_bytes());
    head.extend(48000u32.to_le_bytes());
    head.extend([0, 0, 0]);
    head
}

fn initialization_failure(data: &[u8]) -> TimingError {
    let mut demux = reader(data);
    for _ in 0..100 {
        match demux.poll() {
            Err(DemuxError::Timing(error)) => return error,
            Err(error) => panic!("unexpected initialization error: {error}"),
            Ok(DemuxEvent::TracksReady | DemuxEvent::Packet(_) | DemuxEvent::End(_)) => {
                panic!("invalid initialization exposed tracks, packets or completion")
            }
            _ => (),
        }
    }
    panic!("initialization failed to terminate")
}
#[test]
fn malformed_opus_private_is_rejected_before_tracks_or_packets() {
    let data = document(&opus_track(&[0, 1, 2], Some(0), 2), &[], &[], &[], false);
    assert_eq!(
        initialization_failure(&data).kind,
        TimingErrorKind::OpusHead(unlace::opus::OpusErrorKind::Signature)
    );
}
#[test]
fn conflicting_opus_delay_and_channels_are_not_silently_accepted() {
    for (track, kind) in [
        (
            opus_track(&opus_head(312), Some(7), 2),
            TimingErrorKind::CodecDelayMismatch,
        ),
        (
            opus_track(&opus_head(312), Some(6500000), 1),
            TimingErrorKind::ChannelsMismatch,
        ),
    ] {
        let data = document(&track, &[], &[], &[], false);
        assert_eq!(initialization_failure(&data).kind, kind);
    }
}

fn encoded_offset(data: &[u8], encoded: &[u8]) -> u64 {
    data.windows(encoded.len())
        .position(|window| window == encoded)
        .unwrap() as u64
}
#[test]
fn opus_failures_report_exact_payload_and_conflicting_field_offsets() {
    for (private, relative, kind) in [
        (vec![0, 1, 2], 0, unlace::opus::OpusErrorKind::Signature),
        (
            b"OpusHead".to_vec(),
            8,
            unlace::opus::OpusErrorKind::Truncated,
        ),
        (
            {
                let mut h = opus_head(0);
                h[8] = 16;
                h
            },
            8,
            unlace::opus::OpusErrorKind::UnsupportedVersion,
        ),
        (
            {
                let mut h = opus_head(0);
                h[18] = 2;
                h
            },
            18,
            unlace::opus::OpusErrorKind::UnsupportedMapping,
        ),
    ] {
        let data = document(&opus_track(&private, Some(0), 2), &[], &[], &[], false);
        let element_ = element(0x63a2, &private);
        let start = encoded_offset(&data, &element_) + (element_.len() - private.len()) as u64;
        assert!(
            matches!(drain(&mut reader(&data)),Err(DemuxError::Timing(TimingError {offset,element_id:0x63a2,kind:TimingErrorKind::OpusHead(actual)})) if offset==start+relative && actual==kind)
        );
    }
    for (track, encoded, kind) in [
        (
            opus_track(&opus_head(312), Some(7), 2),
            number(0x56aa, 7),
            TimingErrorKind::CodecDelayMismatch,
        ),
        (
            opus_track(&opus_head(0), Some(0), 1),
            number(0x9f, 1),
            TimingErrorKind::ChannelsMismatch,
        ),
        (
            opus_track(
                &{
                    let mut h = opus_head(0);
                    h[12..16].fill(0);
                    h
                },
                Some(0),
                2,
            ),
            element(0xb5, &48000f64.to_be_bytes()),
            TimingErrorKind::InputRateMismatch,
        ),
    ] {
        let data = document(&track, &[], &[], &[], false);
        let expected = encoded_offset(&data, &encoded);
        assert!(
            matches!(drain(&mut reader(&data)),Err(DemuxError::Timing(TimingError {offset,kind:actual,..})) if offset==expected && actual==kind)
        );
    }
}
#[test]
fn opus_delay_quantization_is_explicit_and_pre_skip_is_exact_samples() {
    for (samples, delay, difference) in [
        (0, 0, 0),
        (1, 20833, -16000),
        (1, 20834, 32000),
        (2, 41666, -32000),
        (2, 41667, 16000),
        (3, 62500, 0),
        (312, 6500000, 0),
        (65535, 1365312500, 0),
    ] {
        let data = document(
            &opus_track(&opus_head(samples), Some(delay), 2),
            &[],
            &[],
            &[],
            false,
        );
        let mut demux = reader(&data);
        drain(&mut demux).unwrap();
        let timing = demux.tracks()[0].opus_timing.unwrap();
        assert_eq!(timing.pre_skip_samples(), samples);
        assert_eq!(
            timing.pre_skip_ns_numerator(),
            u64::from(samples) * 1_000_000_000
        );
        assert_eq!(
            timing.delay_precision(),
            if difference == 0 {
                DelayPrecision::Exact
            } else {
                DelayPrecision::Quantized { difference }
            }
        );
    }
    for delay in [0, 20832, 20835, u64::MAX] {
        let data = document(
            &opus_track(&opus_head(1), Some(delay), 2),
            &[],
            &[],
            &[],
            false,
        );
        assert!(matches!(
            drain(&mut reader(&data)),
            Err(DemuxError::Timing(TimingError {
                kind: TimingErrorKind::CodecDelayMismatch,
                ..
            }))
        ));
    }
}
#[test]
fn scalar_timing_failures_preserve_source_offsets_and_poison_continuation() {
    for (id, bytes) in [(0x2ad7b1, vec![0; 9]), (0x4489, vec![0; 3])] {
        let encoded = element(id, &bytes);
        let data = document(&track(1, true, &[]), &[], &encoded, &[], false);
        let expected = encoded_offset(&data, &encoded);
        let mut demux = reader(&data);
        assert!(
            matches!(drain(&mut demux),Err(DemuxError::Timing(TimingError {offset,element_id,kind:TimingErrorKind::InvalidScalar})) if offset==expected && element_id==id)
        );
        assert!(matches!(demux.poll(), Err(DemuxError::Failed)));
    }
    let mut cluster = number(0xe7, 0);
    let padding = element(0x75a2, &[0; 9]);
    let mut group = element(0xa1, &block(1, 0, 0, &[1]));
    group.extend(&padding);
    cluster.extend(element(0xa0, &group));
    let data = document(
        &track(1, true, &[]),
        &element(CLUSTER, &cluster),
        &[],
        &[],
        false,
    );
    let expected = encoded_offset(&data, &padding);
    assert!(
        matches!(drain(&mut reader(&data)),Err(DemuxError::Timing(TimingError {offset,element_id:0x75a2,kind:TimingErrorKind::InvalidScalar})) if offset==expected)
    );
}
#[test]
fn signed_block_padding_can_span_laced_frames_without_changing_bytes() {
    for padding in [25i64, -25] {
        let mut group = element(0xa1, &block(1, 0, 4, &[1, 7, 8]));
        group.extend(element(0x75a2, &padding.to_be_bytes()));
        let mut cluster = number(0xe7, 0);
        cluster.extend(element(0xa0, &group));
        let data = document(
            &track(1, true, &number(0x23e383, 20)),
            &element(CLUSTER, &cluster),
            &[],
            &[],
            false,
        );
        let mut demux = reader(&data);
        let mut frames = Vec::new();
        let mut notifications = Vec::new();
        loop {
            match demux.poll().unwrap() {
                DemuxEvent::Packet(packet) => {
                    assert_eq!(packet.block_discard_padding_ns, padding);
                    frames.push(packet.data.to_vec());
                    notifications.push(packet.discard_padding_ns);
                }
                DemuxEvent::End(_) => break,
                _ => (),
            }
        }
        assert_eq!(frames, [vec![7], vec![8]]);
        assert_eq!(
            notifications,
            if padding > 0 {
                vec![0, 25]
            } else {
                vec![-25, 0]
            }
        );
    }
}
#[test]
fn padding_beyond_known_whole_block_duration_is_a_located_error() {
    for padding in [41i64, -41, i64::MIN, i64::MAX] {
        let encoded = element(0x75a2, &padding.to_be_bytes());
        let mut group = element(0xa1, &block(1, 0, 4, &[1, 7, 8]));
        group.extend(&encoded);
        let mut cluster = number(0xe7, 0);
        cluster.extend(element(0xa0, &group));
        let data = document(
            &track(1, true, &number(0x23e383, 20)),
            &element(CLUSTER, &cluster),
            &[],
            &[],
            false,
        );
        let expected = encoded_offset(&data, &encoded);
        assert!(
            matches!(drain(&mut reader(&data)),Err(DemuxError::Timing(TimingError {offset,element_id:0x75a2,kind:TimingErrorKind::PaddingExceedsDuration})) if offset==expected)
        );
    }
}

#[test]
fn opus_missing_initialization_and_audio_are_typed_with_enclosing_offsets() {
    for missing_private in [true, false] {
        let mut entry = number(0xd7, 1);
        entry.extend(number(0x73c5, 1));
        entry.extend(number(0x83, 2));
        entry.extend(element(0x86, b"A_OPUS"));
        let (id, kind) = if missing_private {
            let mut audio = element(0xb5, &48000f64.to_be_bytes());
            audio.extend(number(0x9f, 2));
            entry.extend(element(0xe1, &audio));
            (0x63a2, TimingErrorKind::MissingOpusHead)
        } else {
            entry.extend(element(0x63a2, &opus_head(0)));
            (0xe1, TimingErrorKind::MissingAudio)
        };
        let track = element(0xae, &entry);
        let data = document(&track, &[], &[], &[], false);
        let expected = encoded_offset(&data, &track);
        assert!(
            matches!(drain(&mut reader(&data)),Err(DemuxError::Timing(TimingError {offset,element_id,kind:actual})) if offset==expected && element_id==id && actual==kind)
        );
    }
}
#[test]
fn recommended_preroll_is_not_a_mandatory_rejection_and_raw_value_is_preserved() {
    for preroll in [0, 80_000_000, u64::MAX] {
        let full = opus_track(&opus_head(0), Some(0), 2);
        // This short fixture uses one-byte size syntax; rebuild its payload to
        // add the field in a different order without altering an existing field.
        let mut payload = number(0x56bb, preroll);
        payload.extend_from_slice(&full[2..]);
        let data = document(&element(0xae, &payload), &[], &[], &[], false);
        let mut demux = reader(&data);
        drain(&mut demux).unwrap();
        assert_eq!(demux.tracks()[0].seek_preroll_ns, preroll);
        let head = demux.tracks()[0].opus_head().unwrap().unwrap();
        assert_eq!(head.pre_skip_samples(), 0);
    }
}

#[test]
fn opus_validation_survives_wouldblock_at_every_input_position() {
    let mut cluster = number(0xe7, 0);
    cluster.extend(element(0xa3, &block(1, 0, 0, &[1, 2])));
    let data = document(
        &opus_track(&opus_head(312), Some(6_500_000), 2),
        &element(CLUSTER, &cluster),
        &[],
        &[],
        false,
    );
    let expected = drain(&mut reader(&data)).unwrap();
    for position in 0..=data.len() {
        let mut input = source(&data, 1);
        input.fail = Some((position, io::ErrorKind::WouldBlock));
        let mut demux = Demuxer::new(input, AudioSelection::First, DemuxLimits::default()).unwrap();
        assert_eq!(drain(&mut demux).unwrap(), expected);
        assert_eq!(
            demux.tracks()[0].opus_timing.unwrap().pre_skip_samples(),
            312
        );
    }
}
#[test]
fn duplicate_private_and_malformed_preroll_report_the_actual_field_start() {
    let initial = opus_track(&opus_head(0), Some(0), 2);
    for (extra, id, kind) in [
        (
            element(0x63a2, &opus_head(0)),
            0x63a2,
            TimingErrorKind::DuplicateElement,
        ),
        (
            element(0x56bb, &[0; 9]),
            0x56bb,
            TimingErrorKind::InvalidScalar,
        ),
    ] {
        let mut payload = initial[2..].to_vec();
        payload.extend(&extra);
        let data = document(&element(0xae, &payload), &[], &[], &[], false);
        let expected = data
            .windows(extra.len())
            .rposition(|window| window == extra)
            .unwrap() as u64;
        assert!(
            matches!(drain(&mut reader(&data)),Err(DemuxError::Timing(TimingError {offset,element_id,kind:actual})) if offset==expected && element_id==id && actual==kind)
        );
    }
}

#[test]
fn duration_overflow_and_opus_track_type_identify_their_source_fields() {
    let duration = number(0x9b, u64::MAX);
    let mut group = element(0xa1, &block(1, 0, 0, &[1]));
    group.extend(&duration);
    let mut cluster = number(0xe7, 0);
    cluster.extend(element(0xa0, &group));
    let data = document(
        &track(1, true, &[]),
        &element(CLUSTER, &cluster),
        &number(0x2ad7b1, u64::MAX),
        &[],
        false,
    );
    let expected = encoded_offset(&data, &duration);
    assert!(
        matches!(drain(&mut reader(&data)),Err(DemuxError::Timing(TimingError {offset,element_id:0x9b,kind:TimingErrorKind::ArithmeticOverflow})) if offset==expected)
    );
    let mut full = opus_track(&opus_head(0), Some(0), 2);
    let position = encoded_offset(&full, &number(0x83, 2)) as usize;
    full[position + 2] = 1;
    let data = document(&full, &[], &[], &[], false);
    let expected = encoded_offset(&data, &number(0x83, 1));
    assert!(
        matches!(drain(&mut reader(&data)),Err(DemuxError::Timing(TimingError {offset,element_id:0x83,kind:TimingErrorKind::OpusTrackType})) if offset==expected)
    );
}

#[test]
fn opus_exact_delay_rejects_a_whole_nanosecond_difference_in_both_directions() {
    for delay in [6_499_999, 6_500_001] {
        let data = document(
            &opus_track(&opus_head(312), Some(delay), 2),
            &[],
            &[],
            &[],
            false,
        );
        assert!(
            matches!(
                drain(&mut reader(&data)),
                Err(DemuxError::Timing(TimingError {
                    element_id: 0x56aa,
                    kind: TimingErrorKind::CodecDelayMismatch,
                    ..
                }))
            ),
            "accepted delay {delay}"
        );
    }
}

#[test]
fn codec_id_requires_printable_ascii_before_the_null_terminator() {
    for control in [1, 9, 10, 31, 127] {
        let mut t = track(1, true, &[]);
        let start = t.windows(6).position(|b| b == b"A_TEST").unwrap();
        t[start + 5] = control;
        let data = document(&t, &[], &[], &[], false);
        assert!(
            matches!(
                drain(&mut reader(&data)),
                Err(DemuxError::Timing(TimingError {
                    element_id: 0x86,
                    kind: TimingErrorKind::InvalidScalar,
                    ..
                }))
            ),
            "accepted control {control}"
        );
    }
}

#[test]
fn opus_zero_default_is_accepted_but_missing_nonzero_delay_is_not_inferred() {
    let zero = document(&opus_track(&opus_head(0), None, 2), &[], &[], &[], false);
    drain(&mut reader(&zero)).unwrap();
    let t = opus_track(&opus_head(312), None, 2);
    let data = document(&t, &[], &[], &[], false);
    let entry = encoded_offset(&data, &t);
    assert!(
        matches!(drain(&mut reader(&data)), Err(DemuxError::Timing(TimingError {
        offset, element_id: 0x56aa, kind: TimingErrorKind::CodecDelayMismatch
    })) if offset == entry)
    );
}

#[test]
fn printable_ascii_endpoints_and_post_terminator_bytes_preserve_string_semantics() {
    for codec in [b"A_TES ".as_slice(), b"A_TES~", b"A_T\0\x01\xff"] {
        let mut t = track(1, true, &[]);
        let start = t.windows(6).position(|b| b == b"A_TEST").unwrap();
        t[start..start + 6].copy_from_slice(codec);
        let data = document(&t, &[], &[], &[], false);
        let mut demux = reader(&data);
        drain(&mut demux).unwrap();
        assert_eq!(
            demux.tracks()[0].codec_id.as_bytes(),
            codec.split(|&b| b == 0).next().unwrap()
        );
    }
}
