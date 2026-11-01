//! Boundary and exhaustive checks for public EBML integer parsing.
#![allow(clippy::unwrap_used)]

use unlace::{ElementSize, ParseError, Parsed, parse_id, parse_size};

fn complete<T>(value: T, consumed: usize) -> Parsed<T> {
    Parsed::Complete { value, consumed }
}

#[test]
fn size_widths_sentinels_and_every_truncation() {
    assert_eq!(parse_size(&[]), Ok(Parsed::NeedMore { total: 1 }));
    for width in 1..=8 {
        let mut bytes = [0xff; 9];
        bytes[0] = (0xffu16 >> width) as u8 | (1 << (8 - width));
        for n in 1..width {
            assert_eq!(
                parse_size(&bytes[..n]),
                Ok(Parsed::NeedMore { total: width })
            );
        }
        assert_eq!(
            parse_size(&bytes),
            Ok(complete(ElementSize::Unknown, width))
        );
        bytes[width - 1] -= 1;
        let largest = (1u64 << (7 * width)) - 2;
        assert_eq!(
            parse_size(&bytes),
            Ok(complete(ElementSize::Known(largest), width))
        );
        bytes.fill(0);
        bytes[0] = 1 << (8 - width);
        assert_eq!(
            parse_size(&bytes),
            Ok(complete(ElementSize::Known(0), width))
        );
    }
}

#[test]
fn element_ids_preserve_markers_and_current_matroska_rules() {
    for (encoded, id) in [
        (&[0x80][..], 0x80),
        (&[0xfe][..], 0xfe),
        (&[0x40, 0x7f][..], 0x407f),
        (&[0x7f, 0xfe][..], 0x7ffe),
        (&[0x20, 0x3f, 0xff][..], 0x203fff),
        (&[0x10, 0x1f, 0xff, 0xff][..], 0x101fffff),
        (&[0x1a, 0x45, 0xdf, 0xa3][..], 0x1a45dfa3),
    ] {
        let Parsed::Complete { value, consumed } = parse_id(encoded).unwrap() else {
            panic!("expected complete element ID");
        };
        assert_eq!(value.value(), id);
        assert_eq!(consumed, encoded.len());
        for n in 1..encoded.len() {
            assert_eq!(
                parse_id(&encoded[..n]),
                Ok(Parsed::NeedMore {
                    total: encoded.len()
                })
            );
        }
    }
    assert_eq!(parse_id(&[]), Ok(Parsed::NeedMore { total: 1 }));
    for encoded in [
        &[0xff][..],
        &[0x7f, 0xff][..],
        &[0x3f, 0xff, 0xff][..],
        &[0x1f, 0xff, 0xff, 0xff][..],
    ] {
        assert_eq!(parse_id(encoded), Err(ParseError::ReservedElementId));
    }
    for encoded in [
        &[0x40, 0][..],
        &[0x40, 1][..],
        &[0x40, 0x7e][..],
        &[0x20, 0, 0][..],
        &[0x20, 0x3f, 0xfe][..],
        &[0x10, 0x1f, 0xff, 0xfe][..],
    ] {
        assert_eq!(parse_id(encoded), Err(ParseError::NonCanonicalElementId));
    }
}

#[test]
fn invalid_marker_and_width_are_rejected_before_reading_a_payload() {
    assert_eq!(parse_id(&[0]), Err(ParseError::InvalidMarker));
    assert_eq!(parse_size(&[0]), Err(ParseError::InvalidMarker));
    for width in 5u8..=8 {
        assert_eq!(
            parse_id(&[1 << (8 - width)]),
            Err(ParseError::WidthExceeded { width, maximum: 4 })
        );
    }
}

#[test]
fn every_one_and_two_byte_size_matches_the_bit_definition() {
    for raw in 0..=u16::MAX {
        let input = raw.to_be_bytes();
        let first = input[0];
        let parsed = parse_size(&input);
        if first == 0 {
            assert_eq!(parsed, Err(ParseError::InvalidMarker));
            continue;
        }
        let width = first.leading_zeros() as usize + 1;
        if width > input.len() {
            assert_eq!(parsed, Ok(Parsed::NeedMore { total: width }));
        } else {
            let value = if width == 1 {
                u64::from(first & 0x7f)
            } else {
                u64::from(raw & 0x3fff)
            };
            let expected = if value == (1u64 << (7 * width)) - 1 {
                ElementSize::Unknown
            } else {
                ElementSize::Known(value)
            };
            assert_eq!(parsed, Ok(complete(expected, width)));
        }
    }
}

#[test]
fn every_two_byte_id_input_matches_the_registered_ranges() {
    for raw in 0..=u16::MAX {
        let input = raw.to_be_bytes();
        match raw {
            0x0000..=0x00ff => assert_eq!(parse_id(&input), Err(ParseError::InvalidMarker)),
            0x0100..=0x0fff => {
                let width = input[0].leading_zeros() as u8 + 1;
                assert_eq!(
                    parse_id(&input),
                    Err(ParseError::WidthExceeded { width, maximum: 4 })
                );
            }
            0x1000..=0x1fff => assert_eq!(parse_id(&input), Ok(Parsed::NeedMore { total: 4 })),
            0x2000..=0x3fff => assert_eq!(parse_id(&input), Ok(Parsed::NeedMore { total: 3 })),
            0x4000..=0x407e => assert_eq!(parse_id(&input), Err(ParseError::NonCanonicalElementId)),
            0x407f..=0x7ffe => {
                let Parsed::Complete { value, consumed } = parse_id(&input).unwrap() else {
                    panic!("complete ID expected");
                };
                assert_eq!(value.value(), u32::from(raw));
                assert_eq!(consumed, 2);
            }
            0x7fff => assert_eq!(parse_id(&input), Err(ParseError::ReservedElementId)),
            0x8000..=0xfeff => {
                let Parsed::Complete { value, consumed } = parse_id(&input).unwrap() else {
                    panic!("complete ID expected");
                };
                assert_eq!(value.value(), u32::from(input[0]));
                assert_eq!(consumed, 1);
            }
            0xff00..=0xffff => assert_eq!(parse_id(&input), Err(ParseError::ReservedElementId)),
        }
    }
}
