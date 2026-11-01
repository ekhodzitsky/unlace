//! Block framing regressions built from RFC 9559's wire format, without codecs.
#![allow(clippy::unwrap_used)]
use unlace::block::*;

fn vint(value: u64, width: usize) -> Vec<u8> {
    assert!(value < 1u64 << (7 * width));
    let encoded = value | (1u64 << (7 * width));
    encoded.to_be_bytes()[8 - width..].to_vec()
}

fn signed(value: i64) -> Vec<u8> {
    for width in 1..=8 {
        let bias = (1i64 << (7 * width - 1)) - 1;
        if (-bias..=bias + 1).contains(&value) {
            return vint((value + bias) as u64, width);
        }
    }
    panic!("test delta out of range")
}

fn block(mode: Lacing, frames: &[Vec<u8>]) -> Vec<u8> {
    let flags = match mode {
        Lacing::None => 0x80,
        Lacing::Xiph => 0x82,
        Lacing::Fixed => 0x84,
        Lacing::Ebml => 0x86,
    };
    let mut input = vec![0x81, 0xff, 0xfe, flags];
    if mode != Lacing::None {
        input.push((frames.len() - 1) as u8);
        match mode {
            Lacing::Xiph => {
                for frame in &frames[..frames.len() - 1] {
                    let mut length = frame.len();
                    while length >= 255 {
                        input.push(255);
                        length -= 255;
                    }
                    input.push(length as u8);
                }
            }
            Lacing::Ebml => {
                let first = frames[0].len() as u64;
                let width = (1..=8).find(|width| first < 1u64 << (7 * width)).unwrap();
                input.extend(vint(first, width));
                for index in 1..frames.len() - 1 {
                    input.extend(signed(
                        frames[index].len() as i64 - frames[index - 1].len() as i64,
                    ));
                }
            }
            _ => (),
        }
    }
    for frame in frames {
        input.extend(frame);
    }
    input
}

fn parse(input: &[u8]) -> Result<Block<'_>, BlockError> {
    parse_block(input, BlockKind::SimpleBlock, BlockLimits::default())
}

#[test]
fn all_lacing_modes_preserve_exact_frame_bytes_order_and_borrows() {
    for mode in [Lacing::None, Lacing::Fixed, Lacing::Xiph, Lacing::Ebml] {
        let sizes: &[usize] = match mode {
            Lacing::None => &[800],
            Lacing::Fixed => &[800, 800, 800],
            _ => &[800, 500, 1000],
        };
        let frames: Vec<_> = sizes
            .iter()
            .enumerate()
            .map(|(index, &size)| vec![index as u8 + 1; size])
            .collect();
        let input = block(mode, &frames);
        let parsed = parse(&input).unwrap();
        assert_eq!(parsed.track_number(), 1);
        assert_eq!(parsed.relative_timestamp(), -2);
        assert_eq!(parsed.keyframe(), Some(true));
        assert_eq!(parsed.discardable(), Some(false));
        assert!(!parsed.invisible());
        assert_eq!(parsed.lacing(), mode);
        assert_eq!(parsed.frames().len(), frames.len());
        for (view, expected) in parsed.frames().zip(&frames) {
            assert_eq!(view, expected);
            let offset = view.as_ptr() as usize - input.as_ptr() as usize;
            assert_eq!(
                input.get(offset..offset + view.len()).unwrap().as_ptr(),
                view.as_ptr()
            );
        }
        // Independent wire examples from RFC 9559 section 10.3.
        if mode == Lacing::Xiph {
            assert_eq!(&input[4..11], &[2, 255, 255, 255, 35, 255, 245]);
        }
        if mode == Lacing::Ebml {
            assert_eq!(&input[4..9], &[2, 0x43, 0x20, 0x5e, 0xd3]);
        }
    }
}

#[test]
fn track_numbers_support_every_width_including_all_one_data_bits() {
    for width in 1..=8 {
        for track in [1, (1u64 << (7 * width)) - 1] {
            let mut input = vint(track, width);
            input.extend([0x80, 0, 0, 12]);
            let parsed = parse(&input).unwrap();
            assert_eq!(parsed.track_number(), track);
            assert_eq!(parsed.relative_timestamp(), i16::MIN);
            assert_eq!(parsed.frames().collect::<Vec<_>>(), [&[12][..]]);
            for cut in 0..width + 3 {
                assert_eq!(
                    parse(&input[..cut]).unwrap_err(),
                    BlockError::IncompleteHeader
                );
            }
        }
    }
    assert_eq!(parse(&[0x80, 0, 0, 0]).unwrap_err(), BlockError::ZeroTrack);
    assert_eq!(parse(&[0]).unwrap_err(), BlockError::InvalidVint);
}

#[test]
fn flags_have_distinct_simpleblock_and_blockgroup_semantics() {
    let simple = parse(&[0x81, 0x7f, 0xff, 0x89, 1]).unwrap();
    assert_eq!(simple.relative_timestamp(), i16::MAX);
    assert_eq!(simple.keyframe(), Some(true));
    assert_eq!(simple.discardable(), Some(true));
    assert!(simple.invisible());
    for reserved in [0x10, 0x20, 0x40] {
        assert_eq!(
            parse(&[0x81, 0, 0, reserved]).unwrap_err(),
            BlockError::ReservedFlags
        );
    }
    for reserved in [0x10, 0x20, 0x40, 0x80] {
        assert_eq!(
            parse_block(
                &[0x81, 0, 0, reserved],
                BlockKind::Block,
                BlockLimits::default()
            )
            .unwrap_err(),
            BlockError::ReservedFlags
        );
    }
    let group = parse_block(&[0x81, 0, 0, 9], BlockKind::Block, BlockLimits::default()).unwrap();
    assert_eq!(group.keyframe(), None);
    assert_eq!(group.discardable(), None);
    assert_eq!(group.flags(), 9); // unused low bit is preserved, not reinterpreted
    assert!(group.invisible());
}

#[test]
fn every_mode_rejects_lacing_for_one_frame_and_missing_count() {
    for flags in [2, 4, 6] {
        assert_eq!(
            parse(&[0x81, 0, 0, flags]).unwrap_err(),
            BlockError::IncompleteHeader
        );
        assert_eq!(
            parse(&[0x81, 0, 0, flags, 0]).unwrap_err(),
            BlockError::SingleFrameLace
        );
    }
}

#[test]
fn malformed_xiph_ebml_and_fixed_lengths_are_rejected() {
    for input in [
        &[0x81, 0, 0, 2, 1, 255][..],  // unterminated Xiph size
        &[0x81, 0, 0, 6, 1][..],       // missing first EBML size
        &[0x81, 0, 0, 6, 1, 0x40][..], // truncated two-byte size
        &[0x81, 0, 0, 6, 2, 0x81][..], // missing signed delta
    ] {
        assert_eq!(parse(input).unwrap_err(), BlockError::IncompleteHeader);
    }
    for input in [
        &[0x81, 0, 0, 2, 1, 10, 1][..], // size beyond payload
        &[0x81, 0, 0, 6, 1, 0x8a, 1][..],
        &[0x81, 0, 0, 6, 2, 0x81, 0x80][..], // negative next frame size
        &[0x81, 0, 0, 6, 2, 0x81, 1, 0, 0, 0, 0, 0, 0, 0][..], // large negative delta
    ] {
        assert_eq!(parse(input).unwrap_err(), BlockError::InvalidFrameSize);
    }
    assert_eq!(
        parse(&[0x81, 0, 0, 6, 1, 0]).unwrap_err(),
        BlockError::InvalidVint
    );
    assert_eq!(
        parse(&[0x81, 0, 0, 4, 1, 1]).unwrap_err(),
        BlockError::UnequalFixedFrames
    );
}

#[test]
fn lace_vints_accept_all_one_values_and_signed_extremes_without_size_sentinels() {
    for frames in [
        vec![vec![1; 127], vec![2; 1]],
        vec![vec![1; 2], vec![2; 66], vec![3; 1]],
        vec![vec![1; 64], vec![2; 1], vec![3; 1]],
    ] {
        let input = block(Lacing::Ebml, &frames);
        assert_eq!(parse(&input).unwrap().frames().collect::<Vec<_>>(), frames);
    }
    // Eight-byte all-one positive delta cannot fit the tiny complete payload.
    let mut input = vec![0x81, 0, 0, 6, 2, 0x81];
    input.extend(vint((1 << 56) - 1, 8));
    assert_eq!(parse(&input).unwrap_err(), BlockError::InvalidFrameSize);
}

#[test]
fn explicit_resource_limits_refuse_untrusted_sizes_and_xiph_work() {
    let input = block(Lacing::Fixed, &[vec![1; 2], vec![2; 2]]);
    for (limits, error) in [
        (
            BlockLimits {
                max_frames: 0,
                ..BlockLimits::default()
            },
            BlockError::Configuration,
        ),
        (
            BlockLimits {
                max_frames: 257,
                ..BlockLimits::default()
            },
            BlockError::Configuration,
        ),
        (
            BlockLimits {
                max_frames: 1,
                ..BlockLimits::default()
            },
            BlockError::Limit("frame count"),
        ),
        (
            BlockLimits {
                max_block_size: input.len() - 1,
                ..BlockLimits::default()
            },
            BlockError::Limit("block size"),
        ),
        (
            BlockLimits {
                max_frame_size: 1,
                ..BlockLimits::default()
            },
            BlockError::Limit("frame size"),
        ),
        (
            BlockLimits {
                max_lace_header_size: 0,
                ..BlockLimits::default()
            },
            BlockError::Limit("lace header size"),
        ),
    ] {
        assert_eq!(
            parse_block(&input, BlockKind::SimpleBlock, limits).unwrap_err(),
            error
        );
    }
    let limits = BlockLimits {
        max_block_size: input.len(),
        max_frame_size: 2,
        max_frames: 2,
        max_lace_header_size: 1,
    };
    assert!(parse_block(&input, BlockKind::SimpleBlock, limits).is_ok());
    let mut xiph = vec![0x81, 0, 0, 2, 1];
    xiph.extend([255; 10_000]);
    let limits = BlockLimits {
        max_lace_header_size: 10,
        ..BlockLimits::default()
    };
    assert_eq!(
        parse_block(&xiph, BlockKind::SimpleBlock, limits).unwrap_err(),
        BlockError::Limit("lace header size")
    );
    let ebml = block(Lacing::Ebml, &[vec![1; 127], vec![2; 1]]);
    let limits = BlockLimits {
        max_lace_header_size: 1,
        ..BlockLimits::default()
    };
    assert_eq!(
        parse_block(&ebml, BlockKind::SimpleBlock, limits).unwrap_err(),
        BlockError::Limit("lace header size")
    );
}

#[test]
fn empty_frames_and_all_256_frame_slots_are_preserved() {
    for mode in [Lacing::None, Lacing::Xiph, Lacing::Fixed, Lacing::Ebml] {
        let count = if mode == Lacing::None { 1 } else { 256 };
        for length in [0, 1] {
            let frames = vec![vec![7; length]; count];
            let input = block(mode, &frames);
            assert_eq!(parse(&input).unwrap().frames().collect::<Vec<_>>(), frames);
        }
    }
}

#[test]
fn independently_encoded_variable_laces_round_trip_frame_boundaries() {
    for count in 2..=256 {
        let frames: Vec<_> = (0..count)
            .map(|index| vec![index as u8; (index * 487 + count * 37) % 1024])
            .collect();
        for mode in [Lacing::Xiph, Lacing::Ebml] {
            let input = block(mode, &frames);
            let parsed = parse(&input).unwrap();
            assert_eq!(parsed.frames().collect::<Vec<_>>(), frames);
        }
    }
}

#[test]
fn all_small_flag_count_pairs_and_deterministic_random_inputs_remain_bounded() {
    for flags in 0..=255 {
        for count in 0..=255 {
            let input = [0x81, 0, 0, flags, count, 0x80, 0x80, 0x80];
            if let Ok(parsed) = parse(&input) {
                let frames: Vec<_> = parsed.frames().collect();
                assert!(!frames.is_empty() && frames.len() <= 256);
                assert!(frames.iter().map(|frame| frame.len()).sum::<usize>() <= input.len() - 4);
            }
        }
    }
    let mut state = 0x51a2_3bc4_u32;
    for length in 0..64 {
        for _ in 0..200 {
            let mut input = vec![0; length];
            for byte in &mut input {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                *byte = state as u8;
            }
            if let Ok(parsed) = parse(&input) {
                assert!(parsed.frames().map(|frame| frame.len()).sum::<usize>() <= length);
            }
        }
    }
}
