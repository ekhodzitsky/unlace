//! Incremental, schema-driven EBML framing over [`Read`], without seeking.
//!
//! This is a structural core, not a Matroska document validator. The caller
//! supplies a trusted schema and interprets leaf values and document semantics.
//! Unknown IDs fail explicitly; opaque elements must be declared in the schema.
//! Payload bytes go directly into caller storage. No payload is retained.

use crate::{ElementId, ElementSize, ParseError, Parsed, parse_id, parse_size};
use std::{
    fmt,
    io::{self, Read},
};

const STACK_CAPACITY: usize = 64;

/// Where an element may occur in this nonrecursive schema subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parent {
    /// At the stream's root level.
    Root,
    /// Directly inside this master ID.
    Element(u32),
    /// Inside any master or at root; never terminates an unknown-sized master.
    Global,
}

/// Structural handling and the resource category of an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Parse children, optionally permitting unknown size.
    Master {
        /// Schema permission, not a statement about a particular instance.
        unknown_size: bool,
    },
    /// An uninterpreted leaf bounded by the metadata limit.
    Metadata,
    /// An uninterpreted leaf bounded by the payload limit.
    Payload,
    /// Stream past a known finite element without interpreting its contents.
    /// Descendants of an opaque master are deliberately not validated.
    Skip,
}

/// A trusted schema entry. IDs must be canonical and unique.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementSpec {
    /// Encoded EBML ID including its marker.
    pub id: u32,
    /// Permitted parent.
    pub parent: Parent,
    /// Structural handling.
    pub kind: Kind,
}

/// A checked, borrowed table of nonrecursive element relationships.
///
/// This subset has one path per ID and unrestricted occurrence counts. It does
/// not express scalar types, defaults, mandatory children, CRCs or ordering.
/// Schema storage and lookup cost depend on this trusted table, never on input.
#[derive(Clone, Copy, Debug)]
pub struct Schema<'a>(&'a [ElementSpec]);

/// Invalid caller-provided schema, before input is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchemaError;

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid or unsupported EBML schema relationships")
    }
}
impl std::error::Error for SchemaError {}

impl<'a> Schema<'a> {
    /// Validate IDs, uniqueness, master parents, acyclic paths and unknown-size
    /// permissions on ancestors (RFC 8794 section 11.1.6.10).
    pub fn new(entries: &'a [ElementSpec]) -> Result<Self, SchemaError> {
        let schema = Self(entries);
        for (index, entry) in entries.iter().enumerate() {
            let bytes = entry.id.to_be_bytes();
            let start = bytes.iter().position(|&b| b != 0).ok_or(SchemaError)?;
            if !matches!(parse_id(&bytes[start..]), Ok(Parsed::Complete { value, consumed })
                if value.value() == entry.id && consumed == bytes.len() - start)
                || entries[..index].iter().any(|other| other.id == entry.id)
                || (entry.parent == Parent::Global && matches!(entry.kind, Kind::Master { .. }))
            {
                return Err(SchemaError);
            }
            let mut current = *entry;
            for steps in 0..=entries.len() {
                let Parent::Element(id) = current.parent else {
                    break;
                };
                if steps == entries.len() {
                    return Err(SchemaError);
                }
                let parent = schema.find(id).ok_or(SchemaError)?;
                if !matches!(parent.kind, Kind::Master { .. })
                    || (matches!(current.kind, Kind::Master { unknown_size: true })
                        && !matches!(parent.kind, Kind::Master { unknown_size: true }))
                {
                    return Err(SchemaError);
                }
                current = parent;
            }
        }
        Ok(schema)
    }

    fn find(self, id: u32) -> Option<ElementSpec> {
        self.0.iter().find(|entry| entry.id == id).copied()
    }
}

/// Per-element and per-poll limits. No counter grows with recording duration.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum open masters, from 1 through 64.
    pub max_depth: usize,
    /// Maximum declared metadata leaf size in bytes.
    pub max_metadata_size: u64,
    /// Maximum declared payload leaf size in bytes.
    pub max_payload_size: u64,
    /// Maximum declared opaque element size in bytes.
    pub max_skip_size: u64,
    /// Maximum source read calls in a single poll, including interrupted calls.
    pub max_read_calls: usize,
    /// Maximum source bytes consumed in a single poll.
    pub max_read_bytes: usize,
    /// Maximum ID width, from 1 through 4.
    pub max_id_width: u8,
    /// Maximum size width, from 1 through 8.
    pub max_size_width: u8,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: 16,
            max_metadata_size: 1024 * 1024,
            max_payload_size: 16 * 1024 * 1024,
            max_skip_size: 64 * 1024 * 1024,
            max_read_calls: 32,
            max_read_bytes: 64 * 1024,
            max_id_width: 4,
            max_size_width: 8,
        }
    }
}

/// A header with absolute offsets measured from the reader's initial position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// Validated integer syntax; membership is checked against the schema.
    pub id: ElementId,
    /// Absolute offset of the first ID byte.
    pub offset: u64,
    /// Absolute offset of the first payload byte.
    pub data_offset: u64,
    /// Declared payload size.
    pub size: ElementSize,
}

/// Why a master ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    /// Its own finite boundary, or that of its finite ancestor, was reached.
    Boundary,
    /// A schema-valid sibling, ancestor or root ended an unknown-sized master.
    NextElement,
    /// The source reported EOF with no unfinished finite element or header.
    SourceEof,
}

/// One event per poll. Start and End are balanced for leaves as well as masters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A validated structural header. No payload has been read yet.
    Start {
        /// Element header.
        header: Header,
        /// Schema handling.
        kind: Kind,
    },
    /// Leaf bytes in the first `len` bytes of the caller's buffer.
    Data {
        /// Number of initialized bytes; always positive.
        len: usize,
    },
    /// A leaf or master ended. Skipped elements emit Start and End but no Data.
    End {
        /// Original header.
        header: Header,
        /// Observed end condition.
        reason: EndReason,
    },
    /// The source returned WouldBlock. All state is retained; polling may resume.
    NeedInput,
    /// Per-poll byte/read budget exhausted. Poll again to continue.
    Yield,
    /// Final source EOF at a structural boundary. This is not proof that an
    /// unknown-sized recording contains every packet intended by its producer,
    /// or that application-level mandatory elements have been supplied.
    SourceEof,
}

/// A terminal failure. Further polls return [`StreamError::Failed`].
#[derive(Debug)]
pub enum StreamError {
    /// Invalid integer syntax.
    Integer(ParseError),
    /// The schema cannot interpret this ID; no speculative skipping is done.
    UnsupportedId(u32),
    /// Invalid relationship, unknown size, parent overrun or offset overflow.
    Invalid(&'static str),
    /// EOF within a header, finite master or leaf.
    Truncated {
        /// Absolute source offset reached.
        offset: u64,
    },
    /// A declared size or nesting depth exceeds the configured limit.
    Limit(&'static str),
    /// Source error other than Interrupted or WouldBlock, preserving its cause.
    Io(io::Error),
    /// Invalid limits or empty poll buffer. Invalid poll buffers consume no
    /// input and may be retried; invalid construction limits create no reader.
    Configuration,
    /// A preceding terminal error makes continuation unsafe.
    Failed,
}
impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(error) => write!(f, "EBML integer: {error}"),
            Self::UnsupportedId(id) => write!(f, "unsupported EBML element ID {id:#x}"),
            Self::Invalid(reason) => write!(f, "invalid EBML structure: {reason}"),
            Self::Truncated { offset } => write!(f, "truncated EBML input at byte {offset}"),
            Self::Limit(kind) => write!(f, "EBML resource limit: {kind}"),
            Self::Io(error) => write!(f, "EBML source: {error}"),
            Self::Configuration => f.write_str("invalid EBML reader configuration"),
            Self::Failed => f.write_str("EBML reader previously failed"),
        }
    }
}
impl std::error::Error for StreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Integer(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Frame {
    header: Header,
    // Nearest finite boundary, inherited by unknown-sized masters.
    end: Option<u64>,
}
#[derive(Clone, Copy, Debug)]
struct Leaf {
    header: Header,
    remaining: u64,
    skip: bool,
}

/// A bounded event reader owning its source and all continuation state.
///
/// `Read::read` returning zero means final EOF; a growing/nonblocking source
/// must return WouldBlock instead. Interrupted reads consume the read-call
/// budget. Fatal errors poison the reader. There is no resynchronization or
/// portable checkpoint API. A blocking source may still block inside a read;
/// budgets bound calls/bytes, not source latency or work inside `Read`.
pub struct Reader<'a, R> {
    source: R,
    schema: Schema<'a>,
    limits: Limits,
    stack: [Option<Frame>; STACK_CAPACITY],
    depth: usize,
    leaf: Option<Leaf>,
    header_bytes: [u8; 12],
    header_len: usize,
    pending: Option<Header>,
    offset: u64,
    eof: bool,
    failed: bool,
}

impl<'a, R: Read> Reader<'a, R> {
    /// Construct without reading or allocating. Offsets start at zero.
    pub fn new(source: R, schema: Schema<'a>, limits: Limits) -> Result<Self, StreamError> {
        if !(1..=STACK_CAPACITY).contains(&limits.max_depth)
            || limits.max_read_calls == 0
            || limits.max_read_bytes == 0
            || !(1..=4).contains(&limits.max_id_width)
            || !(1..=8).contains(&limits.max_size_width)
        {
            return Err(StreamError::Configuration);
        }
        Ok(Self {
            source,
            schema,
            limits,
            stack: [None; STACK_CAPACITY],
            depth: 0,
            leaf: None,
            header_bytes: [0; 12],
            header_len: 0,
            pending: None,
            offset: 0,
            eof: false,
            failed: false,
        })
    }

    /// Bytes consumed, including any lookahead header retained for the next event.
    pub fn position(&self) -> u64 {
        self.offset
    }

    pub(crate) fn source_mut(&mut self) -> &mut R {
        &mut self.source
    }
    pub(crate) fn source(&self) -> &R {
        &self.source
    }

    // Only the container seek layer may call this, after attesting a real child
    // boundary in this same immutable source and validating Segment metadata.
    pub(crate) fn restart_child(&mut self, parent: Header, offset: u64) -> Result<(), StreamError> {
        let end = match parent.size {
            ElementSize::Known(size) => Some(
                parent
                    .data_offset
                    .checked_add(size)
                    .ok_or(StreamError::Invalid("restart parent overflow"))?,
            ),
            ElementSize::Unknown => None,
        };
        if offset < parent.data_offset || end.is_some_and(|end| offset >= end) {
            return Err(StreamError::Invalid("restart outside parent"));
        }
        self.stack.fill(None);
        self.stack[0] = Some(Frame {
            header: parent,
            end,
        });
        self.depth = 1;
        self.leaf = None;
        self.header_len = 0;
        self.pending = None;
        self.offset = offset;
        self.eof = false;
        self.failed = false;
        Ok(())
    }

    // The document layer applies declared maxima after the finite EBML header
    // ends, before any body header bytes have been read.
    pub(crate) fn set_width_limits(&mut self, id: u8, size: u8) -> Result<(), StreamError> {
        if !(1..=4).contains(&id)
            || !(1..=8).contains(&size)
            || self.header_len != 0
            || self.pending.is_some()
            || self.leaf.is_some()
        {
            return Err(StreamError::Configuration);
        }
        self.limits.max_id_width = id;
        self.limits.max_size_width = size;
        Ok(())
    }

    /// Recover the owned source. Lookahead bytes may already have been consumed;
    /// this is not a restart or checkpoint operation.
    pub fn into_inner(self) -> R {
        self.source
    }

    /// Obtain one event, using a nonempty buffer for data or skipped bytes.
    /// Only Data makes buffer contents meaningful. Payload views remain valid
    /// until the caller changes its buffer; the reader borrows no output storage.
    pub fn poll(&mut self, buffer: &mut [u8]) -> Result<Event, StreamError> {
        if self.failed {
            return Err(StreamError::Failed);
        }
        if buffer.is_empty() {
            return Err(StreamError::Configuration);
        }
        let result = self.poll_inner(buffer);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn top(&self) -> Option<Frame> {
        self.depth
            .checked_sub(1)
            .and_then(|index| self.stack[index])
    }

    fn pop(&mut self, reason: EndReason) -> Result<Event, StreamError> {
        let frame = self
            .top()
            .ok_or(StreamError::Invalid("empty master stack"))?;
        self.depth -= 1;
        self.stack[self.depth] = None;
        Ok(Event::End {
            header: frame.header,
            reason,
        })
    }

    fn poll_inner(&mut self, buffer: &mut [u8]) -> Result<Event, StreamError> {
        let mut calls = 0;
        let mut bytes = 0;
        loop {
            if let Some(mut leaf) = self.leaf {
                if leaf.remaining == 0 {
                    self.leaf = None;
                    return Ok(Event::End {
                        header: leaf.header,
                        reason: EndReason::Boundary,
                    });
                }
                let count = buffer
                    .len()
                    .min(usize::try_from(leaf.remaining).unwrap_or(usize::MAX));
                match self.read(&mut buffer[..count], &mut calls, &mut bytes)? {
                    ReadState::Bytes(len) => {
                        leaf.remaining -= len as u64;
                        self.leaf = Some(leaf);
                        if !leaf.skip {
                            return Ok(Event::Data { len });
                        }
                    }
                    ReadState::Eof => {
                        return Err(StreamError::Truncated {
                            offset: self.offset,
                        });
                    }
                    ReadState::Pending(event) => return Ok(event),
                }
                continue;
            }
            // Use the start of retained lookahead, not the current source offset,
            // when deciding whether an enclosing finite master has ended.
            let boundary_position = self.pending.map_or(self.offset, |header| header.offset);
            if self.header_len == 0
                && self
                    .top()
                    .is_some_and(|frame| frame.end == Some(boundary_position))
            {
                return self.pop(EndReason::Boundary);
            }
            if self.eof {
                if let Some(frame) = self.top() {
                    if frame.end.is_some() {
                        return Err(StreamError::Truncated {
                            offset: self.offset,
                        });
                    }
                    return self.pop(EndReason::SourceEof);
                }
                return Ok(Event::SourceEof);
            }
            let header = if let Some(header) = self.pending {
                header
            } else {
                match self.parse_header()? {
                    Some(header) => {
                        self.pending = Some(header);
                        self.header_len = 0;
                        header
                    }
                    None => {
                        if self
                            .top()
                            .is_some_and(|frame| frame.end == Some(self.offset))
                        {
                            return Err(StreamError::Invalid("header crosses parent boundary"));
                        }
                        let mut byte = [0];
                        match self.read(&mut byte, &mut calls, &mut bytes)? {
                            ReadState::Bytes(_) => {
                                let slot = self
                                    .header_bytes
                                    .get_mut(self.header_len)
                                    .ok_or(StreamError::Invalid("header exceeds twelve bytes"))?;
                                *slot = byte[0];
                                self.header_len += 1;
                            }
                            ReadState::Eof => {
                                if self.header_len != 0 {
                                    return Err(StreamError::Truncated {
                                        offset: self.offset,
                                    });
                                }
                                self.eof = true;
                            }
                            ReadState::Pending(event) => return Ok(event),
                        }
                        continue;
                    }
                }
            };
            let spec = self
                .schema
                .find(header.id.value())
                .ok_or(StreamError::UnsupportedId(header.id.value()))?;
            let actual_parent = self.top().map_or(Parent::Root, |frame| {
                Parent::Element(frame.header.id.value())
            });
            if spec.parent != Parent::Global && spec.parent != actual_parent {
                // Close only unknown masters, and only if the incoming element
                // has a valid parent among the remaining ancestors (or root).
                let can_close =
                    self.top()
                        .is_some_and(|frame| frame.header.size == ElementSize::Unknown)
                        && (spec.parent == Parent::Root
                            || self.stack[..self.depth].iter().flatten().any(|frame| {
                                spec.parent == Parent::Element(frame.header.id.value())
                            }));
                if can_close {
                    return self.pop(EndReason::NextElement);
                }
                return Err(StreamError::Invalid("element has an invalid parent"));
            }
            let end = match header.size {
                ElementSize::Known(size) => Some(
                    header
                        .data_offset
                        .checked_add(size)
                        .ok_or(StreamError::Invalid("element end overflows offset"))?,
                ),
                ElementSize::Unknown => {
                    if !matches!(spec.kind, Kind::Master { unknown_size: true }) {
                        return Err(StreamError::Invalid("unknown size is not permitted"));
                    }
                    None
                }
            };
            let parent_end = self.top().and_then(|frame| frame.end);
            if parent_end.is_some_and(|limit| {
                header.data_offset > limit || end.is_some_and(|end| end > limit)
            }) {
                return Err(StreamError::Invalid("element exceeds parent boundary"));
            }
            match spec.kind {
                Kind::Master { .. } => {
                    if self.depth == self.limits.max_depth {
                        return Err(StreamError::Limit("depth"));
                    }
                    self.stack[self.depth] = Some(Frame {
                        header,
                        end: end.or(parent_end),
                    });
                    self.depth += 1;
                }
                kind => {
                    let ElementSize::Known(size) = header.size else {
                        return Err(StreamError::Invalid("unknown leaf size"));
                    };
                    let (limit, name) = match kind {
                        Kind::Metadata => (self.limits.max_metadata_size, "metadata size"),
                        Kind::Payload => (self.limits.max_payload_size, "payload size"),
                        _ => (self.limits.max_skip_size, "skip size"),
                    };
                    if size > limit {
                        return Err(StreamError::Limit(name));
                    }
                    self.leaf = Some(Leaf {
                        header,
                        remaining: size,
                        skip: kind == Kind::Skip,
                    });
                }
            }
            self.pending = None;
            return Ok(Event::Start {
                header,
                kind: spec.kind,
            });
        }
    }

    fn parse_header(&self) -> Result<Option<Header>, StreamError> {
        let input = &self.header_bytes[..self.header_len];
        let (id, id_width) = match parse_id(input).map_err(StreamError::Integer)? {
            Parsed::NeedMore { total } => {
                if total > usize::from(self.limits.max_id_width) {
                    return Err(StreamError::Limit("ID width"));
                }
                return Ok(None);
            }
            Parsed::Complete { value, consumed } => (value, consumed),
        };
        let (size, size_width) =
            match parse_size(&input[id_width..]).map_err(StreamError::Integer)? {
                Parsed::NeedMore { total } => {
                    if total > usize::from(self.limits.max_size_width) {
                        return Err(StreamError::Limit("size width"));
                    }
                    return Ok(None);
                }
                Parsed::Complete { value, consumed } => (value, consumed),
            };
        Ok(Some(Header {
            id,
            size,
            data_offset: self.offset,
            offset: self.offset - (id_width + size_width) as u64,
        }))
    }

    fn read(
        &mut self,
        output: &mut [u8],
        calls: &mut usize,
        bytes: &mut usize,
    ) -> Result<ReadState, StreamError> {
        loop {
            if *calls == self.limits.max_read_calls || *bytes == self.limits.max_read_bytes {
                return Ok(ReadState::Pending(Event::Yield));
            }
            let count = output.len().min(self.limits.max_read_bytes - *bytes);
            *calls += 1;
            match self.source.read(&mut output[..count]) {
                Ok(0) => return Ok(ReadState::Eof),
                Ok(len) => {
                    if len > count {
                        return Err(StreamError::Invalid(
                            "source returned an invalid read count",
                        ));
                    }
                    self.offset = self
                        .offset
                        .checked_add(len as u64)
                        .ok_or(StreamError::Invalid("source offset overflows"))?;
                    *bytes += len;
                    return Ok(ReadState::Bytes(len));
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(ReadState::Pending(Event::NeedInput));
                }
                Err(error) => return Err(StreamError::Io(error)),
            }
        }
    }
}

enum ReadState {
    Bytes(usize),
    Eof,
    Pending(Event),
}
