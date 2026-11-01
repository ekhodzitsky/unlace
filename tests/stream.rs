//! Structural streaming regressions using independently constructed synthetic EBML.
#![allow(clippy::unwrap_used)] // Test failures should identify the violated assertion.

use std::io::{self, Read};
use unlace::{ElementSize, stream::*};

const SPECS: &[ElementSpec] = &[
    ElementSpec {
        id: 0xa0,
        parent: Parent::Root,
        kind: Kind::Master { unknown_size: true },
    },
    ElementSpec {
        id: 0xa1,
        parent: Parent::Element(0xa0),
        kind: Kind::Master { unknown_size: true },
    },
    ElementSpec {
        id: 0xa2,
        parent: Parent::Element(0xa1),
        kind: Kind::Metadata,
    },
    ElementSpec {
        id: 0xa3,
        parent: Parent::Element(0xa1),
        kind: Kind::Payload,
    },
    ElementSpec {
        id: 0xa4,
        parent: Parent::Element(0xa0),
        kind: Kind::Metadata,
    },
    ElementSpec {
        id: 0xa5,
        parent: Parent::Element(0xa0),
        kind: Kind::Skip,
    },
    ElementSpec {
        id: 0xa6,
        parent: Parent::Element(0xa1),
        kind: Kind::Master {
            unknown_size: false,
        },
    },
    ElementSpec {
        id: 0xa7,
        parent: Parent::Element(0xa6),
        kind: Kind::Metadata,
    },
    ElementSpec {
        id: 0xec,
        parent: Parent::Global,
        kind: Kind::Skip,
    },
];

fn element(id: u8, payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() < 127);
    let mut bytes = vec![id, 0x80 | payload.len() as u8];
    bytes.extend_from_slice(payload);
    bytes
}

fn fixture() -> Vec<u8> {
    let mut cluster = element(0xa2, &[1, 2]);
    // Header-looking bytes inside payloads must never change hierarchy.
    cluster.extend(element(0xa3, &[0xa0, 0xff, 0xa1, 0x80, 0xec, 0xff]));
    cluster.extend(element(0xa6, &element(0xa7, &[9])));
    let mut root = element(0xa1, &cluster);
    root.extend(element(0xa5, &[0, 0xff, 0xa0, 0xff]));
    root.extend(element(0xa4, &[5]));
    element(0xa0, &root)
}

#[derive(Debug, PartialEq, Eq)]
enum Observation {
    Start(u32, u64, ElementSize),
    End(u32, EndReason),
    Byte(u8),
}

fn collect<R: Read>(
    reader: &mut Reader<'_, R>,
    chunk: usize,
) -> Result<Vec<Observation>, StreamError> {
    let mut events = Vec::new();
    let mut buffer = vec![0; chunk];
    for _ in 0..100_000 {
        match reader.poll(&mut buffer)? {
            Event::Start { header, .. } => events.push(Observation::Start(
                header.id.value(),
                header.offset,
                header.size,
            )),
            Event::Data { len } => {
                events.extend(buffer[..len].iter().copied().map(Observation::Byte))
            }
            Event::End { header, reason } => {
                events.push(Observation::End(header.id.value(), reason))
            }
            Event::NeedInput | Event::Yield => (),
            Event::SourceEof => return Ok(events),
        }
    }
    panic!("reader failed to make bounded progress")
}

struct Fragmented<'a> {
    data: &'a [u8],
    position: usize,
    chunk: usize,
    injection: Option<(usize, io::ErrorKind)>,
    calls: usize,
    interrupt_forever: bool,
}
impl<'a> Fragmented<'a> {
    fn new(data: &'a [u8], chunk: usize) -> Self {
        Self {
            data,
            position: 0,
            chunk,
            injection: None,
            calls: 0,
            interrupt_forever: false,
        }
    }
}
impl Read for Fragmented<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.interrupt_forever {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self
            .injection
            .is_some_and(|(position, _)| position == self.position)
        {
            let (_, kind) = self.injection.take().unwrap();
            return Err(io::Error::new(kind, "injected source failure"));
        }
        let count = output
            .len()
            .min(self.chunk)
            .min(self.data.len() - self.position);
        output[..count].copy_from_slice(&self.data[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

fn reader<R: Read>(source: R) -> Reader<'static, R> {
    Reader::new(source, Schema::new(SPECS).unwrap(), Limits::default()).unwrap()
}

#[test]
fn finite_fixture_has_exact_events_bytes_and_offsets() {
    let bytes = fixture();
    let mut stream = reader(bytes.as_slice());
    let events = collect(&mut stream, 64).unwrap();
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                Observation::Byte(b) => Some(*b),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [1, 2, 0xa0, 0xff, 0xa1, 0x80, 0xec, 0xff, 9, 5]
    );
    let ids: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Observation::Start(id, ..) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(ids, [0xa0, 0xa1, 0xa2, 0xa3, 0xa6, 0xa7, 0xa5, 0xa4]);
    let mut stack = Vec::new();
    for event in events {
        match event {
            Observation::Start(id, offset, _) => {
                assert_eq!(u32::from(bytes[offset as usize]), id);
                stack.push(id);
            }
            Observation::End(id, reason) => {
                assert_eq!(stack.pop(), Some(id));
                assert_eq!(reason, EndReason::Boundary);
            }
            _ => (),
        }
    }
    assert!(stack.is_empty());
    assert_eq!(stream.position(), bytes.len() as u64);
    assert_eq!(stream.poll(&mut [0]).unwrap(), Event::SourceEof);
}

#[test]
fn short_reads_and_all_single_byte_interruptions_resume_exactly() {
    let data = fixture();
    let expected = collect(&mut reader(data.as_slice()), 64).unwrap();
    for chunk in 1..=data.len() {
        assert_eq!(
            collect(&mut reader(Fragmented::new(&data, chunk)), chunk).unwrap(),
            expected
        );
    }
    for kind in [io::ErrorKind::Interrupted, io::ErrorKind::WouldBlock] {
        for position in 0..=data.len() {
            let mut source = Fragmented::new(&data, 1);
            source.injection = Some((position, kind));
            assert_eq!(
                collect(&mut reader(source), 1).unwrap(),
                expected,
                "{position} {kind:?}"
            );
        }
    }
}

#[test]
fn every_nonempty_truncation_of_finite_fixture_fails_and_poisoning_is_explicit() {
    let data = fixture();
    for length in 1..data.len() {
        let mut stream = reader(&data[..length]);
        assert!(
            matches!(collect(&mut stream, 3), Err(StreamError::Truncated { offset }) if offset == length as u64),
            "cut {length}"
        );
        assert!(matches!(stream.poll(&mut [0]), Err(StreamError::Failed)));
    }
    assert!(collect(&mut reader(&[][..]), 1).unwrap().is_empty());
}

#[test]
fn source_errors_at_every_header_and_payload_position_keep_their_cause() {
    let data = fixture();
    for position in 0..=data.len() {
        let mut source = Fragmented::new(&data, 1);
        source.injection = Some((position, io::ErrorKind::PermissionDenied));
        let error = collect(&mut reader(source), 1).unwrap_err();
        assert!(
            matches!(error, StreamError::Io(ref error) if error.kind() == io::ErrorKind::PermissionDenied && error.to_string() == "injected source failure")
        );
    }
}

#[test]
fn unknown_masters_close_on_siblings_ancestors_root_and_eof() {
    let data = [
        0xa0, 0xff, 0xa1, 0xff, 0xa2, 0x80, 0xec, 0x80, 0xa1, 0xff, 0xa4, 0x80, 0xa0, 0xff,
    ];
    let events = collect(&mut reader(data.as_slice()), 1).unwrap();
    let ends: Vec<_> = events
        .into_iter()
        .filter_map(|event| match event {
            Observation::End(id, reason) => Some((id, reason)),
            _ => None,
        })
        .collect();
    assert_eq!(
        ends,
        [
            (0xa2, EndReason::Boundary),
            (0xec, EndReason::Boundary),
            (0xa1, EndReason::NextElement),
            (0xa1, EndReason::NextElement),
            (0xa4, EndReason::Boundary),
            (0xa0, EndReason::NextElement),
            (0xa0, EndReason::SourceEof),
        ]
    );
}

#[test]
fn unknown_child_inherits_finite_boundary_without_reading_next_root() {
    let data = [0xa0, 0x84, 0xa1, 0xff, 0xa2, 0x80, 0xa0, 0x80];
    let events = collect(&mut reader(data.as_slice()), 1).unwrap();
    assert_eq!(events[4], Observation::End(0xa1, EndReason::Boundary));
    assert_eq!(events[5], Observation::End(0xa0, EndReason::Boundary));
    assert_eq!(
        events[6],
        Observation::Start(0xa0, 6, ElementSize::Known(0))
    );
}

#[test]
fn invalid_parent_boundaries_and_unknown_forms_fail() {
    let inputs: &[&[u8]] = &[
        &[0xa0, 0x81, 0xa1],       // incomplete child header inside finite master
        &[0xa0, 0x82, 0xa1, 0x81], // child extends beyond parent
        &[0xa0, 0x84, 0xa1, 0xff, 0xa2, 0x81], // leaf exceeds inherited boundary
        &[0xa0, 0xff, 0xa2, 0x80], // wrong direct parent
        &[0xa0, 0x84, 0xa1, 0xff, 0xa0, 0x80], // root cannot terminate finite ancestor
        &[0xa0, 0xff, 0xa4, 0xff], // unknown metadata
        &[0xa0, 0xff, 0xa5, 0xff], // unknown opaque element
        &[0xa0, 0xff, 0xa1, 0xff, 0xa3, 0xff], // unknown payload
        &[0xa0, 0xff, 0xa1, 0xff, 0xa6, 0xff], // disallowed unknown master
        &[0xec, 0xff],             // unknown global
    ];
    for input in inputs {
        assert!(
            matches!(
                collect(&mut reader(*input), 1),
                Err(StreamError::Invalid(_))
            ),
            "{input:x?}"
        );
    }
    assert!(matches!(
        collect(&mut reader(&[0xa8, 0x80][..]), 1),
        Err(StreamError::UnsupportedId(0xa8))
    ));
    assert!(matches!(
        collect(&mut reader(&[0][..]), 1),
        Err(StreamError::Integer(_))
    ));
}

#[test]
fn metadata_payload_skip_and_depth_limits_refuse_before_reading_payloads() {
    for (input, limit) in [
        (&[0xa0, 0xff, 0xa4, 0x81][..], "metadata size"),
        (&[0xa0, 0xff, 0xa1, 0xff, 0xa3, 0x81][..], "payload size"),
        (&[0xa0, 0xff, 0xa5, 0x81][..], "skip size"),
        (&[0xa0, 0xff, 0xa1, 0xff][..], "depth"),
    ] {
        let limits = Limits {
            max_metadata_size: 0,
            max_payload_size: 0,
            max_skip_size: 0,
            max_depth: 1 + usize::from(limit == "payload size"),
            ..Limits::default()
        };
        let mut stream = Reader::new(input, Schema::new(SPECS).unwrap(), limits).unwrap();
        assert!(matches!(collect(&mut stream, 1), Err(StreamError::Limit(name)) if name == limit));
        assert_eq!(stream.position(), input.len() as u64);
    }
    // A maximal finite VINT is refused, never turned into an allocation.
    let data = [
        0xa0, 0xff, 0xa4, 1, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe,
    ];
    assert!(matches!(
        collect(&mut reader(data.as_slice()), 1),
        Err(StreamError::Limit("metadata size"))
    ));
}

#[test]
fn budgets_bound_interrupted_reads_and_skip_work_per_poll() {
    let mut source = Fragmented::new(&[], 1);
    source.interrupt_forever = true;
    let limits = Limits {
        max_read_calls: 3,
        max_read_bytes: 2,
        ..Limits::default()
    };
    let mut stream = Reader::new(source, Schema::new(SPECS).unwrap(), limits).unwrap();
    for _ in 0..10 {
        assert_eq!(stream.poll(&mut [0]).unwrap(), Event::Yield);
    }
    assert_eq!(stream.into_inner().calls, 30);

    let data = element(0xa0, &element(0xa5, &[1; 20]));
    let mut stream = Reader::new(data.as_slice(), Schema::new(SPECS).unwrap(), limits).unwrap();
    loop {
        let before = stream.position();
        let event = stream.poll(&mut [0; 64]).unwrap();
        assert!(stream.position() - before <= 2);
        if event == Event::SourceEof {
            break;
        }
    }
    let expected = collect(&mut reader(fixture().as_slice()), 64).unwrap();
    let fixture = fixture();
    let mut stream = Reader::new(fixture.as_slice(), Schema::new(SPECS).unwrap(), limits).unwrap();
    assert_eq!(collect(&mut stream, 64).unwrap(), expected);
}

#[test]
fn malformed_schemas_are_rejected_before_reading() {
    let master = ElementSpec {
        id: 0xa0,
        parent: Parent::Root,
        kind: Kind::Master {
            unknown_size: false,
        },
    };
    let child = ElementSpec {
        id: 0xa1,
        parent: Parent::Element(0xa0),
        kind: Kind::Master { unknown_size: true },
    };
    assert!(Schema::new(&[master, child]).is_err()); // unknown permission on parent
    assert!(Schema::new(&[master, master]).is_err());
    assert!(Schema::new(&[ElementSpec { id: 0xff, ..master }]).is_err());
    assert!(Schema::new(&[ElementSpec { id: 0, ..master }]).is_err());
    assert!(
        Schema::new(&[ElementSpec {
            id: 0x4001,
            ..master
        }])
        .is_err()
    );
    assert!(
        Schema::new(&[ElementSpec {
            parent: Parent::Global,
            ..master
        }])
        .is_err()
    );
    assert!(
        Schema::new(&[ElementSpec {
            parent: Parent::Element(0xa0),
            ..master
        }])
        .is_err()
    );
    assert!(
        Schema::new(&[
            ElementSpec {
                parent: Parent::Element(0xa1),
                ..master
            },
            ElementSpec {
                kind: master.kind,
                ..child
            }
        ])
        .is_err()
    );
    assert!(Schema::new(&[child]).is_err());
    assert!(
        Schema::new(&[
            ElementSpec {
                kind: Kind::Metadata,
                ..master
            },
            child
        ])
        .is_err()
    );
}

#[test]
fn configuration_errors_are_explicit_and_empty_buffer_is_retryable() {
    let schema = Schema::new(SPECS).unwrap();
    for limits in [
        Limits {
            max_depth: 0,
            ..Limits::default()
        },
        Limits {
            max_depth: 65,
            ..Limits::default()
        },
        Limits {
            max_read_calls: 0,
            ..Limits::default()
        },
        Limits {
            max_read_bytes: 0,
            ..Limits::default()
        },
        Limits {
            max_id_width: 0,
            ..Limits::default()
        },
        Limits {
            max_id_width: 5,
            ..Limits::default()
        },
        Limits {
            max_size_width: 0,
            ..Limits::default()
        },
        Limits {
            max_size_width: 9,
            ..Limits::default()
        },
    ] {
        assert!(matches!(
            Reader::new(&[][..], schema, limits),
            Err(StreamError::Configuration)
        ));
    }
    let mut stream = reader(&[0xa0, 0x80][..]);
    assert!(matches!(
        stream.poll(&mut []),
        Err(StreamError::Configuration)
    ));
    assert_eq!(stream.position(), 0);
    collect(&mut stream, 1).unwrap();
}

#[test]
fn caller_width_limits_reject_wide_integer_prefixes() {
    let schema = Schema::new(SPECS).unwrap();
    let limits = Limits {
        max_id_width: 1,
        max_size_width: 1,
        ..Limits::default()
    };
    let mut stream = Reader::new(&[0x40][..], schema, limits).unwrap();
    assert!(matches!(
        stream.poll(&mut [0]),
        Err(StreamError::Limit("ID width"))
    ));
    let mut stream = Reader::new(&[0xa0, 0x40][..], schema, limits).unwrap();
    assert!(matches!(
        stream.poll(&mut [0]),
        Err(StreamError::Limit("size width"))
    ));
}

#[test]
fn long_streams_have_no_total_byte_or_element_count_limit() {
    let data: Vec<_> = (0..100_000).flat_map(|_| [0xa0, 0x80]).collect();
    let mut stream = reader(data.as_slice());
    let mut starts = 0;
    loop {
        match stream.poll(&mut [0]).unwrap() {
            Event::Start { .. } => starts += 1,
            Event::SourceEof => break,
            _ => (),
        }
    }
    assert_eq!(starts, 100_000);
    assert_eq!(stream.position(), 200_000);
}

#[test]
fn arbitrary_small_inputs_terminate_without_panics_or_unbounded_polls() {
    // Deterministic adversarial smoke test, not a substitute for sustained fuzzing.
    let mut state = 0x7a13_12bd_u32;
    for length in 0..64 {
        for _ in 0..200 {
            let mut data = vec![0; length];
            for byte in &mut data {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                *byte = state as u8;
            }
            let mut stream = reader(data.as_slice());
            let _outcome = collect(&mut stream, 3);
            assert!(stream.position() <= length as u64);
        }
    }
}

#[test]
fn twelve_byte_headers_preserve_offsets_and_all_truncations() {
    let specs = [ElementSpec {
        id: 0x1a45dfa3,
        parent: Parent::Root,
        kind: Kind::Master {
            unknown_size: false,
        },
    }];
    let schema = Schema::new(&specs).unwrap();
    let data = [0x1a, 0x45, 0xdf, 0xa3, 1, 0, 0, 0, 0, 0, 0, 0];
    for length in 1..data.len() {
        let mut stream = Reader::new(&data[..length], schema, Limits::default()).unwrap();
        assert!(
            matches!(collect(&mut stream, 1), Err(StreamError::Truncated { offset }) if offset == length as u64)
        );
    }
    let mut stream = Reader::new(Fragmented::new(&data, 1), schema, Limits::default()).unwrap();
    let Event::Start { header, .. } = stream.poll(&mut [0]).unwrap() else {
        panic!("missing header")
    };
    assert_eq!(header.offset, 0);
    assert_eq!(header.data_offset, 12);
    assert_eq!(header.size, ElementSize::Known(0));
    assert!(matches!(
        stream.poll(&mut [0]).unwrap(),
        Event::End {
            reason: EndReason::Boundary,
            ..
        }
    ));
    assert_eq!(stream.poll(&mut [0]).unwrap(), Event::SourceEof);
}

#[test]
fn maximum_stack_depth_is_usable_and_next_level_refused() {
    let specs: Vec<_> = (0..65)
        .map(|index| ElementSpec {
            id: 0x81 + index,
            parent: if index == 0 {
                Parent::Root
            } else {
                Parent::Element(0x80 + index)
            },
            kind: Kind::Master { unknown_size: true },
        })
        .collect();
    let schema = Schema::new(&specs).unwrap();
    let data: Vec<_> = (0..65).flat_map(|index| [0x81 + index, 0xff]).collect();
    let limits = Limits {
        max_depth: 64,
        ..Limits::default()
    };
    let mut stream = Reader::new(&data[..128], schema, limits).unwrap();
    let events = collect(&mut stream, 1).unwrap();
    assert_eq!(events.len(), 128);
    let mut stream = Reader::new(data.as_slice(), schema, limits).unwrap();
    assert!(matches!(
        collect(&mut stream, 1),
        Err(StreamError::Limit("depth"))
    ));
}

#[test]
fn opaque_known_elements_are_skipped_without_parsing_their_contents() {
    let data = element(0xa0, &element(0xa5, &[0, 0, 0xff, 0xff]));
    let mut stream = reader(data.as_slice());
    let events = collect(&mut stream, 1).unwrap();
    assert_eq!(events.len(), 4);
    assert_eq!(
        events[1],
        Observation::Start(0xa5, 2, ElementSize::Known(4))
    );
    assert_eq!(events[2], Observation::End(0xa5, EndReason::Boundary));
}
