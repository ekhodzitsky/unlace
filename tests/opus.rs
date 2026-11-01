//! Independent wire-format regressions for Opus initialization metadata only.
#![allow(clippy::unwrap_used)]
use unlace::opus::{OpusErrorKind, parse_opus_head};
fn head(channels: u8, family: u8) -> Vec<u8> {
    let mut bytes = b"OpusHead".to_vec();
    bytes.extend([1, channels]);
    bytes.extend(312u16.to_le_bytes());
    bytes.extend(44100u32.to_le_bytes());
    bytes.extend((-256i16).to_le_bytes());
    bytes.push(family);
    bytes
}
#[test]
fn mono_stereo_fields_are_little_endian_borrowed_and_unapplied() {
    for channels in [1, 2] {
        let bytes = head(channels, 0);
        let parsed = parse_opus_head(&bytes).unwrap();
        assert_eq!(parsed.channels(), channels);
        assert_eq!(parsed.pre_skip_samples(), 312);
        assert_eq!(parsed.input_sample_rate(), 44100);
        assert_eq!(parsed.output_gain_q8_db(), -256);
        assert_eq!(parsed.stream_count(), 1);
        assert_eq!(parsed.coupled_stream_count(), channels - 1);
        assert!(parsed.channel_mapping().is_empty());
        assert!(parsed.extension().is_empty());
        assert_eq!(parsed.raw().as_ptr(), bytes.as_ptr());
        assert_eq!(parsed.pre_skip_ns_numerator(), 312_000_000_000);
    }
}
#[test]
fn every_truncation_and_wrong_magic_byte_has_exact_offset() {
    let bytes = head(2, 0);
    for end in 0..bytes.len() {
        let error = parse_opus_head(&bytes[..end]).unwrap_err();
        assert_eq!(error.offset, end);
        assert_eq!(error.kind, OpusErrorKind::Truncated);
    }
    for index in 0..8 {
        let mut invalid = bytes.clone();
        invalid[index] ^= 1;
        let error = parse_opus_head(&invalid).unwrap_err();
        assert_eq!(error.offset, index);
        assert_eq!(error.kind, OpusErrorKind::Signature);
    }
}
#[test]
fn version_compatibility_retains_extensions_without_accepting_unknown_major() {
    for version in 0..=255 {
        let mut bytes = head(1, 0);
        bytes[8] = version;
        if version < 16 {
            assert_eq!(parse_opus_head(&bytes).unwrap().version(), version);
        } else {
            let error = parse_opus_head(&bytes).unwrap_err();
            assert_eq!(error.kind, OpusErrorKind::UnsupportedVersion);
            assert_eq!(error.offset, 8);
        }
        bytes.extend([1, 2, 3]);
        if (2..16).contains(&version) {
            assert_eq!(parse_opus_head(&bytes).unwrap().extension(), [1, 2, 3]);
        } else if version < 2 {
            assert_eq!(
                parse_opus_head(&bytes).unwrap_err().kind,
                OpusErrorKind::TrailingData
            );
        }
    }
}
#[test]
fn multistream_maps_validate_counts_indexes_and_mapping_family() {
    let mut bytes = head(6, 1);
    bytes.extend([4, 2, 0, 4, 1, 2, 3, 5]);
    let parsed = parse_opus_head(&bytes).unwrap();
    assert_eq!(parsed.channel_mapping(), [0, 4, 1, 2, 3, 5]);
    assert_eq!(parsed.stream_count(), 4);
    for end in 19..bytes.len() {
        let error = parse_opus_head(&bytes[..end]).unwrap_err();
        assert_eq!(error.kind, OpusErrorKind::Truncated);
        assert_eq!(error.offset, end);
    }
    let mut invalid = bytes.clone();
    invalid[22] = 6;
    assert_eq!(parse_opus_head(&invalid).unwrap_err().offset, 22);
    invalid[22] = 255;
    assert!(parse_opus_head(&invalid).is_ok());
    for family in 2..255 {
        let mut invalid = head(2, family);
        invalid.extend([1, 1, 0, 1]);
        assert_eq!(
            parse_opus_head(&invalid).unwrap_err().kind,
            OpusErrorKind::UnsupportedMapping
        );
    }
    for channels in [0, 3, 255] {
        assert_eq!(
            parse_opus_head(&head(channels, 0)).unwrap_err().kind,
            OpusErrorKind::Channels
        );
    }
    assert_eq!(
        parse_opus_head(&head(9, 1)).unwrap_err().kind,
        OpusErrorKind::Channels
    );
}
#[test]
fn exhaustive_stream_coupling_counts_obey_decoded_channel_bound() {
    for streams in 0..=255u8 {
        for coupled in 0..=255u8 {
            let mut bytes = head(1, 255);
            bytes.extend([streams, coupled, 255]);
            let valid =
                streams > 0 && coupled <= streams && u16::from(streams) + u16::from(coupled) <= 255;
            assert_eq!(
                parse_opus_head(&bytes).is_ok(),
                valid,
                "{streams} {coupled}"
            );
        }
    }
    let mut bytes = head(255, 255);
    bytes.extend([255, 0]);
    bytes.extend(0..255);
    assert_eq!(
        parse_opus_head(&bytes).unwrap().channel_mapping().len(),
        255
    );
}
#[test]
fn all_pre_skip_values_zero_input_rate_and_gain_endpoints_remain_exact_metadata() {
    let mut bytes = head(1, 0);
    bytes[12..16].fill(0);
    for samples in 0..=u16::MAX {
        bytes[10..12].copy_from_slice(&samples.to_le_bytes());
        let parsed = parse_opus_head(&bytes).unwrap();
        assert_eq!(parsed.pre_skip_samples(), samples);
        assert_eq!(
            parsed.pre_skip_ns_numerator(),
            u64::from(samples) * 1_000_000_000
        );
        assert_eq!(parsed.input_sample_rate(), 0);
    }
    for gain in [i16::MIN, 0, i16::MAX] {
        bytes[16..18].copy_from_slice(&gain.to_le_bytes());
        assert_eq!(parse_opus_head(&bytes).unwrap().output_gain_q8_db(), gain);
    }
}
