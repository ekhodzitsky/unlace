//! Snapshot identity, range continuation, read accounting and bounded I/O.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::rc::Rc;
use unlace::source::{
    ByteExtent, RangeReader, ReadAt, ReadLimits, SeekSource, Snapshot, SourceError,
};

struct State {
    bytes: Vec<u8>,
    snapshot: Snapshot,
    available: usize,
    chunk: usize,
    calls: u64,
    requests: Vec<ByteExtent>,
    interrupt: bool,
    error: bool,
    zero: bool,
    invalid_count: bool,
    change_during_read: bool,
    fail_snapshot: bool,
    fail_after_read: bool,
}
#[derive(Clone)]
struct Memory(Rc<RefCell<State>>);
impl Memory {
    fn new(bytes: &[u8]) -> Self {
        Self(Rc::new(RefCell::new(State {
            bytes: bytes.to_vec(),
            snapshot: Snapshot {
                length: bytes.len() as u64,
                revision: 42,
            },
            available: bytes.len(),
            chunk: usize::MAX,
            calls: 0,
            requests: Vec::new(),
            interrupt: false,
            error: false,
            zero: false,
            invalid_count: false,
            change_during_read: false,
            fail_snapshot: false,
            fail_after_read: false,
        })))
    }
}
impl ReadAt for Memory {
    fn snapshot(&mut self) -> io::Result<Snapshot> {
        let mut state = self.0.borrow_mut();
        if state.fail_snapshot {
            state.fail_snapshot = false;
            return Err(io::Error::other("snapshot failure"));
        }
        Ok(state.snapshot)
    }
    fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        let mut state = self.0.borrow_mut();
        state.calls += 1;
        state.requests.push(ByteExtent {
            start: offset,
            end: offset + buffer.len() as u64,
        });
        if state.fail_after_read {
            state.fail_snapshot = true;
            state.fail_after_read = false;
        }
        if state.change_during_read {
            state.snapshot.revision += 1;
        }
        if state.interrupt {
            state.interrupt = false;
            return Err(io::ErrorKind::Interrupted.into());
        }
        if state.error {
            state.error = false;
            return Err(io::Error::other("read failure"));
        }
        if state.zero {
            return Ok(0);
        }
        if state.invalid_count {
            return Ok(buffer.len() + 1);
        }
        let offset = usize::try_from(offset).unwrap();
        if offset >= state.available {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let len = buffer.len().min(state.chunk).min(state.available - offset);
        buffer[..len].copy_from_slice(&state.bytes[offset..offset + len]);
        Ok(len)
    }
}

#[test]
fn backward_reads_preserve_bytes_and_count_actual_work() {
    let memory = Memory::new(b"0123456789");
    memory.0.borrow_mut().chunk = 2;
    let mut input = RangeReader::new(memory.clone(), ReadLimits::default()).unwrap();
    let mut buffer = [0; 8];
    assert_eq!(
        input.snapshot(),
        Snapshot {
            length: 10,
            revision: 42
        }
    );
    for (offset, expected) in [
        (7, &b"78"[..]),
        (0, &b"01"[..]),
        (2, &b"23"[..]),
        (0, &b"01"[..]),
        (9, &b"9"[..]),
    ] {
        let len = input.read_at(offset, &mut buffer).unwrap();
        assert_eq!(&buffer[..len], expected);
    }
    assert_eq!(input.report().bytes, 9);
    assert_eq!(input.report().calls, 5);
    assert_eq!(
        input.report().extents,
        [
            ByteExtent { start: 7, end: 9 },
            ByteExtent { start: 0, end: 4 },
            ByteExtent { start: 0, end: 2 },
            ByteExtent { start: 9, end: 10 }
        ]
    );
    assert!(input.report().extents_complete);
    assert_eq!(memory.0.borrow().calls, 5);
    assert_eq!(input.into_inner().0.borrow().bytes, b"0123456789");
}

#[test]
fn ranges_resume_at_every_available_prefix_without_loss() {
    let bytes = b"original encoded packet bytes";
    for split in 0..=bytes.len() {
        let memory = Memory::new(bytes);
        memory.0.borrow_mut().available = split;
        memory.0.borrow_mut().chunk = 3;
        let mut input = RangeReader::new(memory.clone(), ReadLimits::default()).unwrap();
        let mut output = Vec::new();
        let mut buffer = [0; 5];
        loop {
            let offset = output.len() as u64;
            match input.read_at(offset, &mut buffer) {
                Ok(0) => break,
                Ok(len) => output.extend_from_slice(&buffer[..len]),
                Err(SourceError::NeedRange(extent)) => {
                    assert_eq!(
                        extent,
                        ByteExtent {
                            start: offset,
                            end: (offset + 5).min(bytes.len() as u64)
                        }
                    );
                    assert_eq!(output.len(), split);
                    memory.0.borrow_mut().available = bytes.len();
                }
                result => panic!("unexpected result: {result:?}"),
            }
        }
        assert_eq!(output, bytes);
        assert_eq!(input.report().bytes, bytes.len() as u64);
        assert_eq!(
            input.report().extents,
            [ByteExtent {
                start: 0,
                end: bytes.len() as u64
            }]
        );
        assert!(input.report().extents_complete);
    }
}

#[test]
fn repeated_interrupts_and_unavailable_ranges_consume_call_allowance() {
    let memory = Memory::new(b"abc");
    memory.0.borrow_mut().available = 0;
    let mut input = RangeReader::new(
        memory.clone(),
        ReadLimits {
            max_calls: 3,
            ..ReadLimits::default()
        },
    )
    .unwrap();
    let mut buffer = [0; 10];
    for _ in 0..2 {
        memory.0.borrow_mut().interrupt = true;
        assert!(
            matches!(input.read_at(0, &mut buffer), Err(SourceError::Io { offset: Some(0), error }) if error.kind() == io::ErrorKind::Interrupted)
        );
    }
    assert!(matches!(
        input.read_at(0, &mut buffer),
        Err(SourceError::NeedRange(ByteExtent { start: 0, end: 3 }))
    ));
    assert!(matches!(
        input.read_at(0, &mut buffer),
        Err(SourceError::Limit)
    ));
    assert_eq!(input.report().calls, 3);
    assert_eq!(input.report().bytes, 0);
    assert_eq!(memory.0.borrow().calls, 3);
    input.begin_operation().unwrap();
    memory.0.borrow_mut().available = 3;
    assert_eq!(input.read_at(0, &mut buffer).unwrap(), 3);
    assert_eq!(&buffer[..3], b"abc");
    assert_eq!(input.report().calls, 1);
}

#[test]
fn byte_budget_caps_requests_and_eof_needs_no_remaining_allowance() {
    let memory = Memory::new(b"abcdef");
    let mut input = RangeReader::new(
        memory.clone(),
        ReadLimits {
            max_bytes: 3,
            ..ReadLimits::default()
        },
    )
    .unwrap();
    let mut buffer = [0; 20];
    assert_eq!(input.read_at(1, &mut buffer).unwrap(), 3);
    assert_eq!(&buffer[..3], b"bcd");
    assert_eq!(
        memory.0.borrow().requests,
        [ByteExtent { start: 1, end: 4 }]
    );
    assert!(matches!(
        input.read_at(4, &mut buffer),
        Err(SourceError::Limit)
    ));
    assert_eq!(input.read_at(6, &mut buffer).unwrap(), 0);
    assert_eq!(input.read_at(2, &mut []).unwrap(), 0);
    assert!(matches!(
        input.read_at(7, &mut []),
        Err(SourceError::OutOfBounds { offset: 7 })
    ));
    assert_eq!(input.report().calls, 1);
    input.begin_operation().unwrap();
    assert_eq!(input.report().bytes, 0);
    assert!(input.report().extents.is_empty());
    assert!(input.report().extents_complete);
    assert_eq!(input.read_at(4, &mut buffer).unwrap(), 2);
    assert_eq!(&buffer[..2], b"ef");
}

#[test]
fn missing_range_is_capped_to_budget_not_caller_buffer() {
    let memory = Memory::new(b"abcdef");
    memory.0.borrow_mut().available = 0;
    let mut input = RangeReader::new(
        memory,
        ReadLimits {
            max_bytes: 2,
            ..ReadLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        input.read_at(1, &mut [0; 100]),
        Err(SourceError::NeedRange(ByteExtent { start: 1, end: 3 }))
    ));
    assert_eq!(input.report().bytes, 0);
    assert_eq!(input.report().calls, 1);
}

#[test]
fn revision_and_length_changes_latch_even_when_original_token_returns() {
    for change_length in [false, true] {
        let memory = Memory::new(b"abc");
        let original = memory.0.borrow().snapshot;
        let mut input = RangeReader::new(memory.clone(), ReadLimits::default()).unwrap();
        input.read_at(0, &mut [0]).unwrap();
        if change_length {
            memory.0.borrow_mut().snapshot.length += 1;
        } else {
            memory.0.borrow_mut().snapshot.revision += 1;
            memory.0.borrow_mut().bytes.copy_from_slice(b"xyz");
        }
        assert!(matches!(
            input.read_at(1, &mut [0]),
            Err(SourceError::SourceChanged)
        ));
        memory.0.borrow_mut().snapshot = original;
        memory.0.borrow_mut().bytes.copy_from_slice(b"abc");
        assert!(matches!(
            input.check_snapshot(),
            Err(SourceError::SourceChanged)
        ));
        assert!(matches!(
            input.begin_operation(),
            Err(SourceError::SourceChanged)
        ));
        assert!(matches!(
            input.read_at(0, &mut []),
            Err(SourceError::SourceChanged)
        ));
        assert_eq!(input.report().calls, 1);
        assert_eq!(input.report().bytes, 1);
        assert_eq!(input.snapshot(), original);
    }
}

#[test]
fn change_during_read_rejects_success_and_error_but_retains_io_accounting() {
    for error_return in [false, true] {
        let memory = Memory::new(b"abc");
        memory.0.borrow_mut().change_during_read = true;
        memory.0.borrow_mut().error = error_return;
        let mut input = RangeReader::new(memory, ReadLimits::default()).unwrap();
        assert!(matches!(
            input.read_at(0, &mut [0; 2]),
            Err(SourceError::SourceChanged)
        ));
        assert_eq!(input.report().calls, 1);
        assert_eq!(input.report().bytes, if error_return { 0 } else { 2 });
        assert_eq!(input.report().extents.is_empty(), error_return);
    }
}

#[test]
fn provider_contract_failures_poison_continuation() {
    for invalid_count in [false, true] {
        let memory = Memory::new(b"abc");
        memory.0.borrow_mut().invalid_count = invalid_count;
        memory.0.borrow_mut().zero = !invalid_count;
        let mut input = RangeReader::new(memory.clone(), ReadLimits::default()).unwrap();
        for _ in 0..2 {
            let error = input.read_at(1, &mut [0]).unwrap_err();
            if invalid_count {
                assert!(matches!(error, SourceError::InvalidRead { offset: 1 }));
            } else {
                assert!(matches!(error, SourceError::UnexpectedEof { offset: 1 }));
            }
        }
        assert!(input.begin_operation().is_err());
        assert_eq!(input.report().bytes, 0);
        assert_eq!(input.report().calls, 1);
        assert_eq!(memory.0.borrow().calls, 1);
    }
}

#[test]
fn log_truncation_never_coalesces_across_omitted_reads() {
    for max_extents in [0, 1, 2] {
        let mut input = RangeReader::new(
            SeekSource::new(Cursor::new(b"0123456789"), 0),
            ReadLimits {
                max_extents,
                ..ReadLimits::default()
            },
        )
        .unwrap();
        for offset in [0, 5, 1, 2] {
            assert_eq!(input.read_at(offset, &mut [0]).unwrap(), 1);
        }
        assert_eq!(input.report().bytes, 4);
        assert_eq!(input.report().calls, 4);
        assert!(!input.report().extents_complete);
        assert_eq!(input.report().extents.len(), max_extents);
        if max_extents > 0 {
            assert_eq!(input.report().extents[0], ByteExtent { start: 0, end: 1 });
        }
        input.begin_operation().unwrap();
        assert!(input.report().extents_complete);
    }
}

#[test]
fn io_errors_remain_typed_and_retry_is_explicit() {
    let memory = Memory::new(b"abc");
    let mut input = RangeReader::new(memory.clone(), ReadLimits::default()).unwrap();
    memory.0.borrow_mut().error = true;
    let error = input.read_at(2, &mut [0]).unwrap_err();
    assert!(std::error::Error::source(&error).is_some());
    assert!(matches!(
        error,
        SourceError::Io {
            offset: Some(2),
            ..
        }
    ));
    assert_eq!(input.report().calls, 1);
    memory.0.borrow_mut().fail_snapshot = true;
    assert!(matches!(
        input.begin_operation(),
        Err(SourceError::Io { offset: None, .. })
    ));
    assert_eq!(input.report().calls, 1);
    assert_eq!(input.read_at(2, &mut [0]).unwrap(), 1);
    assert_eq!(input.report().calls, 2);
    memory.0.borrow_mut().fail_snapshot = true;
    assert!(matches!(
        RangeReader::new(memory, ReadLimits::default()),
        Err(SourceError::Io { offset: None, .. })
    ));
}

struct Sparse;
impl ReadAt for Sparse {
    fn snapshot(&mut self) -> io::Result<Snapshot> {
        Ok(Snapshot {
            length: u64::MAX,
            revision: u128::MAX,
        })
    }
    fn read_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        assert_eq!(offset, u64::MAX - 1);
        assert_eq!(buffer.len(), 1);
        buffer[0] = 7;
        Ok(1)
    }
}
#[test]
fn maximum_offsets_do_not_wrap_or_request_beyond_snapshot() {
    let mut input = RangeReader::new(Sparse, ReadLimits::default()).unwrap();
    let mut buffer = [0; 20];
    assert_eq!(input.read_at(u64::MAX - 1, &mut buffer).unwrap(), 1);
    assert_eq!(buffer[0], 7);
    assert_eq!(input.read_at(u64::MAX, &mut buffer).unwrap(), 0);
    assert_eq!(
        input.report().extents,
        [ByteExtent {
            start: u64::MAX - 1,
            end: u64::MAX
        }]
    );
}

#[test]
fn invalid_configuration_does_not_touch_provider() {
    for limits in [
        ReadLimits {
            max_bytes: 0,
            ..ReadLimits::default()
        },
        ReadLimits {
            max_calls: 0,
            ..ReadLimits::default()
        },
    ] {
        let memory = Memory::new(b"a");
        memory.0.borrow_mut().fail_snapshot = true;
        assert!(matches!(
            RangeReader::new(memory.clone(), limits),
            Err(SourceError::Configuration)
        ));
        assert!(memory.0.borrow().fail_snapshot);
    }
}

struct WrongSeek(Cursor<Vec<u8>>);
impl Read for WrongSeek {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0.read(buffer)
    }
}
impl Seek for WrongSeek {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        match position {
            SeekFrom::Start(offset) => Ok(offset + 1),
            _ => self.0.seek(position),
        }
    }
}
#[test]
fn seek_adapter_checks_actual_position() {
    let source = SeekSource::new(WrongSeek(Cursor::new(b"abc".to_vec())), 0);
    let mut input = RangeReader::new(source, ReadLimits::default()).unwrap();
    assert!(
        matches!(input.read_at(0, &mut [0]), Err(SourceError::Io { error, .. }) if error.kind() == io::ErrorKind::InvalidData)
    );
    let cursor = input.into_inner().into_inner().0;
    assert_eq!(cursor.position(), 3); // snapshot seeks end, even after bad read.
}

#[test]
fn failed_post_read_snapshot_withholds_bytes_and_keeps_work_visible() {
    let memory = Memory::new(b"abc");
    memory.0.borrow_mut().fail_after_read = true;
    let mut input = RangeReader::new(memory, ReadLimits::default()).unwrap();
    let mut buffer = [0; 2];
    assert!(matches!(
        input.read_at(0, &mut buffer),
        Err(SourceError::Io { offset: None, .. })
    ));
    // Buffer changes on failure are explicitly allowed, never published as Ok.
    assert_eq!(&buffer, b"ab");
    assert_eq!(input.report().bytes, 2);
    assert_eq!(input.report().calls, 1);
    assert_eq!(input.read_at(0, &mut buffer).unwrap(), 2);
    assert_eq!(input.report().bytes, 4);
    assert_eq!(
        input.report().extents,
        [
            ByteExtent { start: 0, end: 2 },
            ByteExtent { start: 0, end: 2 }
        ]
    );
}

#[derive(Clone)]
struct SharedCursor(Rc<RefCell<Cursor<Vec<u8>>>>);
impl Read for SharedCursor {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0.borrow_mut().read(buffer)
    }
}
impl Seek for SharedCursor {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.0.borrow_mut().seek(position)
    }
}
#[test]
fn seek_adapter_detects_observed_length_change() {
    let cursor = SharedCursor(Rc::new(RefCell::new(Cursor::new(b"abc".to_vec()))));
    let mut input =
        RangeReader::new(SeekSource::new(cursor.clone(), 7), ReadLimits::default()).unwrap();
    cursor.0.borrow_mut().get_mut().push(4);
    assert!(matches!(
        input.read_at(0, &mut [0]),
        Err(SourceError::SourceChanged)
    ));
    assert_eq!(cursor.0.borrow_mut().get_mut().pop(), Some(4));
    assert!(matches!(
        input.check_snapshot(),
        Err(SourceError::SourceChanged)
    ));
    assert_eq!(input.report().calls, 0);
}

#[test]
fn empty_snapshot_is_clean_without_a_provider_read() {
    let mut input = RangeReader::new(Memory::new(b""), ReadLimits::default()).unwrap();
    assert_eq!(input.read_at(0, &mut [0; 10]).unwrap(), 0);
    assert_eq!(input.report().calls, 0);
    assert!(input.report().extents_complete);
    assert!(matches!(
        input.read_at(1, &mut [0]),
        Err(SourceError::OutOfBounds { offset: 1 })
    ));
}
