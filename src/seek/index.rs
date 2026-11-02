use super::{Fallback, Preparation, SeekError, SeekLimits, SeekRequest, SeekResult, cancelled};
use crate::source::{ByteExtent, RangeReader, ReadAt, SourceError};
use crate::stream::Header;
use crate::{ElementSize, Parsed, parse_id, parse_size};
use std::io;
use std::sync::atomic::AtomicBool;

const CLUSTER: u32 = 0x1f43b675;
const CUES: u32 = 0x1c53bb6b;
const SEEKHEAD: u32 = 0x114d9b74;
fn invalid(offset: u64, reason: &'static str) -> SeekError {
    SeekError::Invalid { offset, reason }
}
fn top(id: u32) -> bool {
    matches!(
        id,
        SEEKHEAD | CUES | CLUSTER | 0x1549a966 | 0x1654ae6b | 0x1254c367 | 0x1043a770 | 0x1941a469
    )
}
fn cluster_child(id: u32) -> bool {
    matches!(id, 0xe7 | 0xa7 | 0xab | 0xa3 | 0xa0 | 0xec)
}
fn end(header: Header, bound: u64) -> Result<u64, SeekError> {
    let ElementSize::Known(size) = header.size else {
        return Err(invalid(header.offset, "unexpected unknown size"));
    };
    let end = header
        .data_offset
        .checked_add(size)
        .ok_or_else(|| invalid(header.offset, "element size overflow"))?;
    if end > bound {
        return Err(invalid(header.offset, "element exceeds source/parent"));
    }
    Ok(end)
}
fn reserve<T>(output: &mut Vec<T>, max: usize) -> Result<(), SeekError> {
    if output.len() >= max {
        return Err(SeekError::Limit("index entries"));
    }
    output
        .try_reserve(1)
        .map_err(|_| SeekError::Limit("index allocation"))
}
struct Work<'a> {
    remaining: u64,
    cancel: &'a AtomicBool,
}
impl Work<'_> {
    fn step(&mut self) -> Result<(), SeekError> {
        self.charge(1)
    }
    fn charge(&mut self, count: u64) -> Result<(), SeekError> {
        cancelled(self.cancel)?;
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or(SeekError::Limit("scan headers"))?;
        Ok(())
    }
}
fn read_some<S: ReadAt>(
    input: &mut RangeReader<S>,
    offset: u64,
    buffer: &mut [u8],
    cancel: &AtomicBool,
) -> Result<usize, SeekError> {
    loop {
        cancelled(cancel)?;
        match input.read_at(offset, buffer) {
            Ok(0) => return Err(invalid(offset, "truncated element")),
            Ok(len) => return Ok(len),
            Err(SourceError::Io { error, .. }) if error.kind() == io::ErrorKind::Interrupted => (),
            Err(error) => return Err(error.into()),
        }
    }
}
fn exact<S: ReadAt>(
    input: &mut RangeReader<S>,
    mut offset: u64,
    mut buffer: &mut [u8],
    cancel: &AtomicBool,
) -> Result<(), SeekError> {
    while !buffer.is_empty() {
        let len = read_some(input, offset, buffer, cancel)?;
        offset += len as u64;
        buffer = &mut buffer[len..];
    }
    Ok(())
}
fn header<S: ReadAt>(
    input: &mut RangeReader<S>,
    offset: u64,
    bound: u64,
    size_width: u8,
    cancel: &AtomicBool,
) -> Result<Header, SeekError> {
    let mut bytes = [0; 12];
    let mut len = 0;
    loop {
        let (needed, minimum) =
            match parse_id(&bytes[..len]).map_err(|_| invalid(offset, "invalid element ID"))? {
                // A complete ID always needs at least one size byte. This
                // groups only a valid header's minimum, never payload bytes.
                Parsed::NeedMore { total } => (total, total + 1),
                Parsed::Complete {
                    value: id,
                    consumed,
                } => {
                    match parse_size(&bytes[consumed..len])
                        .map_err(|_| invalid(offset, "invalid size"))?
                    {
                        Parsed::NeedMore { total } => {
                            if total > usize::from(size_width) {
                                return Err(invalid(offset, "declared size width exceeded"));
                            }
                            (consumed + total, consumed + total)
                        }
                        Parsed::Complete {
                            value: size,
                            consumed: width,
                        } => {
                            if width > usize::from(size_width) {
                                return Err(invalid(offset, "declared size width exceeded"));
                            }
                            return Ok(Header {
                                id,
                                offset,
                                data_offset: offset + len as u64,
                                size,
                            });
                        }
                    }
                }
            };
        if offset
            .checked_add(needed as u64)
            .is_none_or(|end| end > bound)
        {
            return Err(invalid(offset, "truncated header/parent boundary"));
        }
        let requested = minimum.min((bound - offset).min(bytes.len() as u64) as usize);
        len += read_some(
            input,
            offset + len as u64,
            &mut bytes[len..requested],
            cancel,
        )?;
    }
}
#[derive(Clone, Copy)]
struct Entry {
    header: Header,
    end: u64,
    // Valid only when Index::cached_track was committed after a successful seek.
    // None then means the whole Cluster scan found no selected-track block.
    cached_first: Option<Block>,
    cached_steps: u64,
}
impl Entry {
    fn new(header: Header, end: u64) -> Self {
        Self {
            header,
            end,
            cached_first: None,
            cached_steps: 0,
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct Cue {
    time: u64,
    track: u64,
    cluster: u64,
    relative: Option<u64>,
    state: u64,
    offset: u64,
}
// Bounded catalogue. No byte-pattern search or following untrusted pointers.
pub(super) struct Index {
    segment: Header,
    end: u64,
    size_width: u8,
    position: u64,
    unknown: Option<(Header, u64)>,
    entries: Vec<Entry>,
    boundaries_done: bool,
    parsed: usize,
    bytes: u64,
    elements: usize,
    cues: Vec<Cue>,
    has_cues: bool,
    cached_track: Option<u64>,
}
pub(super) struct Timing {
    pub track: u64,
    pub delay: u64,
    pub scale: u64,
    pub preroll: u64,
}
impl Timing {
    fn ns(&self, ticks: i128, offset: u64) -> Result<i128, SeekError> {
        ticks
            .checked_mul(i128::from(self.scale))
            .and_then(|n| n.checked_sub(i128::from(self.delay)))
            .ok_or_else(|| invalid(offset, "timestamp overflow"))
    }
}
impl Index {
    pub fn new(segment: Header, size_width: u8, length: u64) -> Result<Self, SeekError> {
        let bound = match segment.size {
            ElementSize::Known(_) => end(segment, length)?,
            ElementSize::Unknown => length,
        };
        if bound != length {
            return Err(SeekError::Unsupported("data following the Segment"));
        }
        Ok(Self {
            segment,
            end: bound,
            size_width,
            position: segment.data_offset,
            unknown: None,
            entries: Vec::new(),
            boundaries_done: false,
            parsed: 0,
            bytes: 0,
            elements: 0,
            cues: Vec::new(),
            has_cues: false,
            cached_track: None,
        })
    }
    pub fn ready(&self) -> bool {
        self.boundaries_done && self.parsed == self.entries.len()
    }
    pub fn validate_tracks(&self, tracks: &[crate::demux::Track]) -> Result<(), SeekError> {
        for cue in &self.cues {
            if !tracks.iter().any(|track| track.number == cue.track) {
                return Err(invalid(cue.offset, "CueTrack is not declared in Tracks"));
            }
        }
        Ok(())
    }
    fn add(&mut self, entry: Entry, max: usize) -> Result<(), SeekError> {
        if !matches!(entry.header.id.value(), CLUSTER | SEEKHEAD | 0xec)
            && self
                .entries
                .iter()
                .any(|other| other.header.id == entry.header.id)
        {
            return Err(invalid(entry.header.offset, "duplicate Segment singleton"));
        }
        reserve(&mut self.entries, max)?;
        self.entries.push(entry);
        Ok(())
    }
    pub fn prepare<S: ReadAt>(
        &mut self,
        input: &mut RangeReader<S>,
        limits: SeekLimits,
        cancel: &AtomicBool,
    ) -> Result<Preparation, SeekError> {
        for _ in 0..limits.max_prepare_steps {
            cancelled(cancel)?;
            if !self.boundaries_done {
                if let Some((cluster, position)) = self.unknown {
                    if position == self.end {
                        self.add(Entry::new(cluster, position), limits.max_entries)?;
                        self.unknown = None;
                        self.position = position;
                        continue;
                    }
                    let child = header(input, position, self.end, self.size_width, cancel)?;
                    if top(child.id.value()) {
                        self.add(Entry::new(cluster, position), limits.max_entries)?;
                        self.unknown = None;
                        self.position = position;
                    } else if cluster_child(child.id.value()) {
                        self.unknown = Some((cluster, end(child, self.end)?));
                    } else {
                        return Err(invalid(position, "unsupported unknown Cluster child"));
                    }
                    continue;
                }
                if self.position == self.end {
                    self.boundaries_done = true;
                    continue;
                }
                let next = header(input, self.position, self.end, self.size_width, cancel)?;
                let id = next.id.value();
                if !top(id) && id != 0xec {
                    return Err(invalid(next.offset, "unsupported Segment child"));
                }
                if id == CLUSTER && next.size == ElementSize::Unknown {
                    self.unknown = Some((next, next.data_offset));
                } else {
                    let finish = end(next, self.end)?;
                    self.add(Entry::new(next, finish), limits.max_entries)?;
                    self.position = finish;
                }
                continue;
            }
            let Some(entry) = self.entries.get(self.parsed).copied() else {
                input.check_snapshot()?;
                cancelled(cancel)?;
                return Ok(Preparation::Ready);
            };
            if matches!(entry.header.id.value(), CUES | SEEKHEAD) {
                let len = entry.end - entry.header.data_offset;
                let total = self
                    .bytes
                    .checked_add(len)
                    .filter(|&total| total <= limits.max_index_bytes)
                    .ok_or(SeekError::Limit("index bytes"))?;
                let len = usize::try_from(len).map_err(|_| SeekError::Limit("index allocation"))?;
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(len)
                    .map_err(|_| SeekError::Limit("index allocation"))?;
                bytes.resize(len, 0);
                exact(input, entry.header.data_offset, &mut bytes, cancel)?;
                let mut parser = Parser {
                    count: self.elements,
                    max: limits.max_index_elements,
                    max_tracks: limits.max_positions_per_cue,
                    size_width: self.size_width,
                    cancel,
                };
                if entry.header.id.value() == CUES {
                    let cues = parser.cues(
                        &bytes,
                        entry.header.data_offset,
                        limits.max_entries - self.cues.len(),
                    )?;
                    for cue in &cues {
                        self.cluster(cue.cluster)?;
                    }
                    self.cues
                        .try_reserve(cues.len())
                        .map_err(|_| SeekError::Limit("index allocation"))?;
                    self.cues.extend(cues);
                    self.has_cues = true;
                } else {
                    for target in
                        parser.seeks(&bytes, entry.header.data_offset, limits.max_entries)?
                    {
                        let offset = self
                            .segment
                            .data_offset
                            .checked_add(target.1)
                            .ok_or_else(|| invalid(entry.header.offset, "SeekPosition overflow"))?;
                        let found = self.entry(offset).ok_or_else(|| {
                            invalid(offset, "SeekPosition is not a Segment child boundary")
                        })?;
                        if found.header.id.value() != target.0 {
                            return Err(invalid(offset, "SeekID does not match destination"));
                        }
                    }
                }
                self.elements = parser.count;
                self.bytes = total;
            }
            self.parsed += 1;
        }
        Ok(Preparation::Yield)
    }
    fn entry(&self, offset: u64) -> Option<Entry> {
        self.entries
            .binary_search_by_key(&offset, |entry| entry.header.offset)
            .ok()
            .and_then(|index| self.entries.get(index))
            .copied()
    }
    fn cluster(&self, position: u64) -> Result<Entry, SeekError> {
        let offset = self
            .segment
            .data_offset
            .checked_add(position)
            .ok_or_else(|| invalid(self.segment.offset, "CueClusterPosition overflow"))?;
        self.entry(offset)
            .filter(|entry| entry.header.id.value() == CLUSTER)
            .ok_or_else(|| invalid(offset, "cue is not a real Cluster boundary"))
    }
    pub fn commit_fallback(&mut self, track: u64) {
        self.cached_track = Some(track);
    }
    pub fn locate<S: ReadAt>(
        &mut self,
        input: &mut RangeReader<S>,
        timing: Timing,
        request: SeekRequest,
        limits: SeekLimits,
        cancel: &AtomicBool,
    ) -> Result<(SeekResult, bool), SeekError> {
        let cached = self.cached_track == Some(timing.track);
        let preroll = request.preroll_ns.max(timing.preroll);
        let target = request
            .timestamp_ns
            .checked_sub(i128::from(preroll))
            .ok_or(SeekError::RequestOverflow)?;
        let mut chosen = None;
        let mut track_cues = false;
        for cue in &self.cues {
            cancelled(cancel)?;
            if cue.track != timing.track {
                continue;
            }
            track_cues = true;
            let ns = timing.ns(i128::from(cue.time), cue.offset)?;
            if ns <= target && chosen.is_none_or(|old: Cue| cue.time > old.time) {
                chosen = Some(*cue);
            }
        }
        let mut work = Work {
            remaining: limits.max_scan_headers,
            cancel,
        };
        let (cluster, first, fallback) = if let Some(cue) = chosen {
            if cue.state != 0 {
                return Err(SeekError::Unsupported("CueCodecState"));
            }
            let cluster = self.cluster(cue.cluster)?;
            let summary = scan_optional(
                input,
                cluster,
                self.size_width,
                timing.track,
                Some(cue),
                &mut work,
            )?
            .ok_or_else(|| invalid(cluster.header.offset, "cue Cluster has no requested track"))?;
            (cluster, summary, None)
        } else {
            let fallback = if !self.has_cues {
                Fallback::MissingCues
            } else if !track_cues {
                Fallback::NoTrackCues
            } else {
                Fallback::BeforeFirstCue
            };
            // A partial or failed scan must never become reusable. Invalidate
            // the old key before staging another track's summaries.
            if !cached {
                self.cached_track = None;
            }
            let mut before: Option<(Entry, Block)> = None;
            let mut earliest: Option<(Entry, Block)> = None;
            for cluster in self
                .entries
                .iter_mut()
                .filter(|entry| entry.header.id.value() == CLUSTER)
            {
                let first = if cached {
                    // Reuse validation, but retain its original scan-work
                    // charge. ReadReport continues to record actual I/O only.
                    work.charge(cluster.cached_steps)?;
                    cluster.cached_first
                } else {
                    let remaining = work.remaining;
                    let first = scan_optional(
                        input,
                        *cluster,
                        self.size_width,
                        timing.track,
                        None,
                        &mut work,
                    )?;
                    cluster.cached_first = first;
                    cluster.cached_steps = remaining - work.remaining;
                    first
                };
                if let Some(first) = first {
                    let ns = timing.ns(first.ticks, first.offset)?;
                    if earliest.is_none_or(|(_, old)| first.ticks < old.ticks) {
                        earliest = Some((*cluster, first));
                    }
                    if ns <= target && before.is_none_or(|(_, old)| first.ticks > old.ticks) {
                        before = Some((*cluster, first));
                    }
                }
            }
            let (cluster, first) = before.or(earliest).ok_or(SeekError::NoAccessPoint)?;
            (cluster, first, Some(fallback))
        };
        cancelled(cancel)?;
        input.check_snapshot()?;
        Ok((
            SeekResult {
                requested_ns: request.timestamp_ns,
                declared_preroll_ns: timing.preroll,
                required_preroll_ns: preroll,
                preroll_target_ns: target,
                actual_timestamp_ns: timing.ns(first.ticks, first.offset)?,
                starts_before_preroll_target: timing.ns(first.ticks, first.offset)? <= target,
                cluster_offset: cluster.header.offset,
                block_offset: first.offset,
                track_number: timing.track,
                cue_time_ticks: chosen.map(|cue| cue.time),
                fallback,
                skipped_prefix: ByteExtent {
                    start: self.segment.data_offset,
                    end: cluster.header.offset,
                },
            },
            fallback.is_some() && !cached,
        ))
    }
}

#[derive(Clone, Copy)]
struct Block {
    offset: u64,
    ticks: i128,
}
fn scan_optional<S: ReadAt>(
    input: &mut RangeReader<S>,
    cluster: Entry,
    width: u8,
    track: u64,
    cue: Option<Cue>,
    work: &mut Work<'_>,
) -> Result<Option<Block>, SeekError> {
    let mut position = cluster.header.data_offset;
    let mut timestamp = None;
    let mut first = None;
    let mut found = false;
    let relative = cue
        .and_then(|cue| cue.relative)
        .map(|offset| {
            cluster
                .header
                .data_offset
                .checked_add(offset)
                .ok_or_else(|| invalid(cluster.header.offset, "CueRelativePosition overflow"))
        })
        .transpose()?;
    while position < cluster.end {
        work.step()?;
        let child = header(input, position, cluster.end, width, work.cancel)?;
        let finish = end(child, cluster.end)?;
        match child.id.value() {
            0xe7 => {
                if timestamp.is_some() {
                    return Err(invalid(position, "duplicate Cluster Timestamp"));
                }
                let len = finish - child.data_offset;
                if len > 8 {
                    return Err(invalid(position, "Cluster Timestamp width"));
                }
                let mut bytes = [0; 8];
                exact(
                    input,
                    child.data_offset,
                    &mut bytes[8 - len as usize..],
                    work.cancel,
                )?;
                timestamp = Some(u64::from_be_bytes(bytes));
            }
            0xa3 | 0xa0 => {
                let base = timestamp
                    .ok_or_else(|| invalid(position, "block precedes Cluster Timestamp"))?;
                let block = if child.id.value() == 0xa0 {
                    group_block(input, child, finish, width, work)?
                } else {
                    child
                };
                let (number, relative_time) = block_prefix(input, block, finish, work.cancel)?;
                if number == track {
                    let current = Block {
                        offset: block.offset,
                        ticks: i128::from(base) + i128::from(relative_time),
                    };
                    if first.is_none() {
                        first = Some(current);
                    }
                    if let Some(cue) = cue {
                        if relative.is_none_or(|offset| offset == position)
                            && current.ticks == i128::from(cue.time)
                        {
                            found = true;
                        }
                    } else {
                        return Ok(first);
                    }
                }
            }
            0xa7 | 0xab | 0xec => (),
            _ => return Err(invalid(position, "unsupported Cluster child")),
        }
        position = finish;
    }
    if timestamp.is_none() {
        return Err(invalid(cluster.header.offset, "missing Cluster Timestamp"));
    }
    if cue.is_some() && !found {
        return Err(invalid(
            cluster.header.offset,
            "cue time/relative position/track does not match a block",
        ));
    }
    Ok(first)
}
fn group_block<S: ReadAt>(
    input: &mut RangeReader<S>,
    group: Header,
    finish: u64,
    width: u8,
    work: &mut Work<'_>,
) -> Result<Header, SeekError> {
    let mut position = group.data_offset;
    let mut block = None;
    while position < finish {
        work.step()?;
        let child = header(input, position, finish, width, work.cancel)?;
        match child.id.value() {
            0xa1 => {
                if block.replace(child).is_some() {
                    return Err(invalid(position, "duplicate Block"));
                }
            }
            0x9b | 0xfb | 0x75a2 | 0xfa | 0x75a1 | 0xec => (),
            0xa4 => return Err(SeekError::Unsupported("codec state changes")),
            _ => return Err(invalid(position, "unsupported BlockGroup child")),
        }
        position = end(child, finish)?;
    }
    block.ok_or_else(|| invalid(group.offset, "BlockGroup lacks Block"))
}
fn block_prefix<S: ReadAt>(
    input: &mut RangeReader<S>,
    block: Header,
    bound: u64,
    cancel: &AtomicBool,
) -> Result<(u64, i16), SeekError> {
    let finish = end(block, bound)?;
    if finish - block.data_offset < 4 {
        return Err(invalid(block.offset, "short Block"));
    }
    let mut bytes = [0; 11];
    // Every valid Block prefix needs at least four bytes. Ask for that
    // minimum together, then inspect the first byte before requesting more.
    // Short reads still validate the marker and declared bounds immediately.
    let read = read_some(input, block.data_offset, &mut bytes[..4], cancel)?;
    if bytes[0] == 0 {
        return Err(invalid(block.offset, "invalid Block track VINT"));
    }
    let width = bytes[0].leading_zeros() as usize + 1;
    let total = width + 3;
    if finish - block.data_offset < total as u64 {
        return Err(invalid(block.offset, "short Block prefix"));
    }
    exact(
        input,
        block.data_offset + read as u64,
        &mut bytes[read..total],
        cancel,
    )?;
    let mut track = u64::from(bytes[0]) & (0xffu64 >> width);
    for byte in &bytes[1..width] {
        track = track * 256 + u64::from(*byte);
    }
    if track == 0 {
        return Err(invalid(block.offset, "zero Block track"));
    }
    Ok((track, i16::from_be_bytes([bytes[width], bytes[width + 1]])))
}

struct Parser<'a> {
    max_tracks: usize,
    count: usize,
    max: usize,
    size_width: u8,
    cancel: &'a AtomicBool,
}
#[derive(Clone, Copy)]
struct Item<'a> {
    id: u32,
    offset: u64,
    data_offset: u64,
    bytes: &'a [u8],
}
impl Parser<'_> {
    fn next<'a>(&mut self, bytes: &mut &'a [u8], offset: &mut u64) -> Result<Item<'a>, SeekError> {
        cancelled(self.cancel)?;
        if self.count >= self.max {
            return Err(SeekError::Limit("index elements"));
        }
        self.count += 1;
        let Parsed::Complete {
            value: id,
            consumed: id_width,
        } = parse_id(bytes).map_err(|_| invalid(*offset, "index ID"))?
        else {
            return Err(invalid(*offset, "truncated index ID"));
        };
        let Parsed::Complete {
            value: ElementSize::Known(size),
            consumed: size_width,
        } = parse_size(&bytes[id_width..]).map_err(|_| invalid(*offset, "index size"))?
        else {
            return Err(invalid(*offset, "truncated/unknown index size"));
        };
        if size_width > usize::from(self.size_width) {
            return Err(invalid(*offset, "index declared size width exceeded"));
        }
        let prefix = id_width + size_width;
        let size = usize::try_from(size).map_err(|_| invalid(*offset, "index size overflow"))?;
        let finish = prefix
            .checked_add(size)
            .filter(|&end| end <= bytes.len())
            .ok_or_else(|| invalid(*offset, "index child overrun"))?;
        if id.value() == 0xbf {
            return Err(SeekError::Unsupported("CRC-32 verification"));
        }
        let item = Item {
            id: id.value(),
            offset: *offset,
            data_offset: *offset + prefix as u64,
            bytes: &bytes[prefix..finish],
        };
        *bytes = &bytes[finish..];
        *offset += finish as u64;
        Ok(item)
    }
    fn cues(
        &mut self,
        mut bytes: &[u8],
        mut offset: u64,
        max: usize,
    ) -> Result<Vec<Cue>, SeekError> {
        let mut output = Vec::new();
        while !bytes.is_empty() {
            let point = self.next(&mut bytes, &mut offset)?;
            if point.id == 0xec {
                continue;
            }
            if point.id != 0xbb {
                return Err(invalid(point.offset, "unsupported Cues child"));
            }
            let mut children = point.bytes;
            let mut position = point.data_offset;
            let mut time = None;
            let begin = output.len();
            while !children.is_empty() {
                let child = self.next(&mut children, &mut position)?;
                match child.id {
                    0xb3 => unique(&mut time, integer(child)?, child.offset)?,
                    0xb7 => {
                        if output.len() - begin >= self.max_tracks {
                            return Err(SeekError::Limit("CuePoint track positions"));
                        }
                        let cue = self.cue_position(child)?;
                        if output[begin..]
                            .iter()
                            .any(|old: &Cue| old.track == cue.track)
                        {
                            return Err(invalid(child.offset, "duplicate CueTrack"));
                        }
                        reserve(&mut output, max)?;
                        output.push(cue);
                    }
                    0xec => (),
                    _ => return Err(invalid(child.offset, "unsupported CuePoint child")),
                }
            }
            let time = time.ok_or_else(|| invalid(point.offset, "missing CueTime"))?;
            if begin == output.len() {
                return Err(invalid(point.offset, "missing CueTrackPositions"));
            }
            for cue in &mut output[begin..] {
                cue.time = time;
            }
        }
        if output.is_empty() {
            return Err(invalid(offset, "empty Cues"));
        }
        Ok(output)
    }
    fn cue_position(&mut self, item: Item<'_>) -> Result<Cue, SeekError> {
        let mut bytes = item.bytes;
        let mut offset = item.data_offset;
        let (mut track, mut cluster, mut relative, mut state, mut duration, mut block_number) =
            (None, None, None, None, None, None);
        while !bytes.is_empty() {
            let child = self.next(&mut bytes, &mut offset)?;
            let output = match child.id {
                0xf7 => &mut track,
                0xf1 => &mut cluster,
                0xf0 => &mut relative,
                0xea => &mut state,
                0xb2 => &mut duration,
                0x5378 => &mut block_number,
                0xec => continue,
                _ => return Err(SeekError::Unsupported("CueTrackPositions extension")),
            };
            unique(output, integer(child)?, child.offset)?;
        }
        let track = track
            .filter(|&track| track != 0)
            .ok_or_else(|| invalid(item.offset, "missing/zero CueTrack"))?;
        if block_number == Some(0) {
            return Err(invalid(item.offset, "zero CueBlockNumber"));
        }
        // Block-number/duration hints never position the reader. Actual track,
        // time and optional relative destination are verified by a boundary walk.
        Ok(Cue {
            time: 0,
            track,
            cluster: cluster.ok_or_else(|| invalid(item.offset, "missing CueClusterPosition"))?,
            relative,
            state: state.unwrap_or(0),
            offset: item.offset,
        })
    }
    fn seeks(
        &mut self,
        mut bytes: &[u8],
        mut offset: u64,
        max: usize,
    ) -> Result<Vec<(u32, u64)>, SeekError> {
        let mut output = Vec::new();
        while !bytes.is_empty() {
            let seek = self.next(&mut bytes, &mut offset)?;
            if seek.id == 0xec {
                continue;
            }
            if seek.id != 0x4dbb {
                return Err(invalid(seek.offset, "unsupported SeekHead child"));
            }
            let mut children = seek.bytes;
            let mut position = seek.data_offset;
            let (mut id, mut target) = (None, None);
            while !children.is_empty() {
                let child = self.next(&mut children, &mut position)?;
                match child.id {
                    0x53ab => {
                        let Parsed::Complete { value, consumed } = parse_id(child.bytes)
                            .map_err(|_| invalid(child.offset, "SeekID syntax"))?
                        else {
                            return Err(invalid(child.offset, "short SeekID"));
                        };
                        if consumed != child.bytes.len() || !top(value.value()) {
                            return Err(invalid(child.offset, "SeekID is not one top-level ID"));
                        }
                        unique(&mut id, value.value(), child.offset)?;
                    }
                    0x53ac => unique(&mut target, integer(child)?, child.offset)?,
                    0xec => (),
                    _ => return Err(invalid(child.offset, "unsupported Seek child")),
                }
            }
            reserve(&mut output, max)?;
            output.push((
                id.ok_or_else(|| invalid(seek.offset, "missing SeekID"))?,
                target.ok_or_else(|| invalid(seek.offset, "missing SeekPosition"))?,
            ));
        }
        if output.is_empty() {
            return Err(invalid(offset, "empty SeekHead"));
        }
        Ok(output)
    }
}
fn integer(item: Item<'_>) -> Result<u64, SeekError> {
    if item.bytes.len() > 8 {
        return Err(invalid(item.offset, "index integer width"));
    }
    Ok(item
        .bytes
        .iter()
        .fold(0, |value, byte| value * 256 + u64::from(*byte)))
}
fn unique<T>(slot: &mut Option<T>, value: T, offset: u64) -> Result<(), SeekError> {
    if slot.replace(value).is_some() {
        return Err(invalid(offset, "duplicate index singleton"));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod grouped_read_tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    type Requests = Rc<RefCell<Vec<(u64, usize)>>>;
    use crate::source::{ReadLimits, Snapshot};
    struct Provider {
        bytes: Vec<u8>,
        chunk: usize,
        calls: Requests,
        stall: Option<Rc<Cell<bool>>>,
    }
    impl ReadAt for Provider {
        fn snapshot(&mut self) -> io::Result<Snapshot> {
            Ok(Snapshot {
                length: self.bytes.len() as u64,
                revision: 1,
            })
        }
        fn read_at(&mut self, offset: u64, out: &mut [u8]) -> io::Result<usize> {
            self.calls.borrow_mut().push((offset, out.len()));
            if out.len() > 1 && self.stall.as_ref().is_some_and(|s| s.get()) {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            let start = offset as usize;
            let n = out.len().min(self.chunk).min(self.bytes.len() - start);
            out[..n].copy_from_slice(&self.bytes[start..start + n]);
            Ok(n)
        }
    }
    fn input(bytes: Vec<u8>, chunk: usize) -> (RangeReader<Provider>, Requests) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        (
            RangeReader::new(
                Provider {
                    bytes,
                    chunk,
                    calls: calls.clone(),
                    stall: None,
                },
                ReadLimits::default(),
            )
            .unwrap(),
            calls,
        )
    }
    #[test]
    fn exact_header_offsets_and_no_payload_requests_at_all_widths() {
        let cancel = AtomicBool::new(false);
        for id in [
            &[0xec][..],
            &[0x4d, 0xbb][..],
            &[0x2a, 0xd7, 0xb1][..],
            &[0x1f, 0x43, 0xb6, 0x75][..],
        ] {
            for width in 1..=8 {
                let mut encoded = vec![0; 5];
                encoded.extend_from_slice(id);
                let mut size = vec![0; width];
                size[0] = 1 << (8 - width);
                size[width - 1] |= 3;
                encoded.extend(size);
                let hlen = encoded.len();
                encoded.extend([0xaa, 0xbb, 0xcc, 0xdd]);
                for chunk in [1, 2, 3, usize::MAX] {
                    let (mut source, calls) = input(encoded.clone(), chunk);
                    let h = header(&mut source, 5, encoded.len() as u64, 8, &cancel).unwrap();
                    assert_eq!(h.offset, 5);
                    assert_eq!(h.data_offset, hlen as u64);
                    assert_eq!(h.size, ElementSize::Known(3));
                    assert!(
                        calls
                            .borrow()
                            .iter()
                            .all(|(at, n)| at + *n as u64 <= hlen as u64)
                    );
                }
            }
        }
    }
    #[test]
    fn truncated_parent_never_requests_beyond_its_boundary() {
        let cancel = AtomicBool::new(false);
        let bytes = vec![0x1f, 0x43, 0xb6, 0x75, 0x01, 0, 0, 0, 0, 0, 0, 0, 0xaa];
        for bound in 0..12u64 {
            for chunk in [1, 2, usize::MAX] {
                let (mut source, calls) = input(bytes.clone(), chunk);
                assert!(matches!(
                    header(&mut source, 0, bound, 8, &cancel),
                    Err(SeekError::Invalid { .. })
                ));
                assert!(calls.borrow().iter().all(|(at, n)| at + *n as u64 <= bound));
            }
        }
    }
    #[test]
    fn successful_short_first_reads_preserve_invalid_marker_priority() {
        let cancel = AtomicBool::new(false);
        for first in [0, 0xff, 0x08] {
            let (mut source, calls) = input(vec![first, 0xff, 0xff, 0xff], 1);
            assert!(matches!(
                header(&mut source, 0, 4, 8, &cancel),
                Err(SeekError::Invalid { .. })
            ));
            assert_eq!(calls.borrow().len(), 1);
        }
        let Parsed::Complete { value: id, .. } = parse_id(&[0xa3]).unwrap() else {
            panic!()
        };
        let (mut source, calls) = input(vec![0, 0xaa, 0xbb, 0xcc], 1);
        let h = Header {
            id,
            offset: 0,
            data_offset: 0,
            size: ElementSize::Known(4),
        };
        assert!(matches!(
            block_prefix(&mut source, h, 4, &cancel),
            Err(SeekError::Invalid { .. })
        ));
        assert_eq!(calls.borrow().len(), 1);
    }

    #[test]
    fn whole_request_unavailability_still_rejects_bad_markers_after_retry() {
        let cancel = AtomicBool::new(false);
        for block in [false, true] {
            let stall = Rc::new(Cell::new(true));
            let calls = Rc::new(RefCell::new(Vec::new()));
            let mut source = RangeReader::new(
                Provider {
                    bytes: vec![0, 0xaa, 0xbb, 0xcc],
                    chunk: usize::MAX,
                    calls,
                    stall: Some(stall.clone()),
                },
                ReadLimits::default(),
            )
            .unwrap();
            let Parsed::Complete { value: id, .. } = parse_id(&[0xa3]).unwrap() else {
                panic!()
            };
            let h = Header {
                id,
                offset: 0,
                data_offset: 0,
                size: ElementSize::Known(4),
            };
            let result = if block {
                block_prefix(&mut source, h, 4, &cancel).map(|_| ())
            } else {
                header(&mut source, 0, 4, 8, &cancel).map(|_| ())
            };
            match result {
                Err(SeekError::Invalid { offset: 0, .. }) => (),
                Err(SeekError::Source(SourceError::NeedRange(_))) => {
                    stall.set(false);
                    let result = if block {
                        block_prefix(&mut source, h, 4, &cancel).map(|_| ())
                    } else {
                        header(&mut source, 0, 4, 8, &cancel).map(|_| ())
                    };
                    assert!(matches!(result, Err(SeekError::Invalid { offset: 0, .. })));
                }
                other => panic!("bad marker accepted or wrong error: {other:?}"),
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod cached_work_tests {
    use super::*;
    use crate::source::{ReadLimits, Snapshot};
    use std::{cell::Cell, rc::Rc};
    struct Provider {
        data: Vec<u8>,
        calls: Rc<Cell<u64>>,
    }
    impl ReadAt for Provider {
        fn snapshot(&mut self) -> io::Result<Snapshot> {
            Ok(Snapshot {
                length: self.data.len() as u64,
                revision: 1,
            })
        }
        fn read_at(&mut self, offset: u64, out: &mut [u8]) -> io::Result<usize> {
            self.calls.set(self.calls.get() + 1);
            let at = offset as usize;
            let len = out.len().min(self.data.len() - at);
            out[..len].copy_from_slice(&self.data[at..at + len]);
            Ok(len)
        }
    }
    fn element(id: u32, data: &[u8]) -> Vec<u8> {
        assert!(data.len() < 127);
        let raw = id.to_be_bytes();
        let start = raw.iter().position(|&v| v != 0).unwrap();
        let mut out = raw[start..].to_vec();
        out.push(0x80 | data.len() as u8);
        out.extend_from_slice(data);
        out
    }
    fn fixture() -> (Index, RangeReader<Provider>, Rc<Cell<u64>>) {
        let mut body = Vec::new();
        for (time, track) in [(100u16, 2u8), (200, 3), (300, 2)] {
            let mut child = element(0xe7, &time.to_be_bytes());
            child.extend(element(0xa3, &[0x80 | track, 0, 0, 0x80, 0x55]));
            body.extend(element(CLUSTER, &child));
        }
        let data = element(0x18538067, &body);
        let calls = Rc::new(Cell::new(0));
        let mut source = RangeReader::new(
            Provider {
                data,
                calls: calls.clone(),
            },
            ReadLimits::default(),
        )
        .unwrap();
        let length = source.snapshot().length;
        let cancel = AtomicBool::new(false);
        let segment = header(&mut source, 0, length, 8, &cancel).unwrap();
        let mut index = Index::new(segment, 8, length).unwrap();
        while index
            .prepare(&mut source, SeekLimits::default(), &cancel)
            .unwrap()
            != Preparation::Ready
        {}
        (index, source, calls)
    }
    fn timing(track: u64) -> Timing {
        Timing {
            track,
            delay: 7,
            scale: 1000,
            preroll: 0,
        }
    }
    fn request() -> SeekRequest {
        SeekRequest {
            timestamp_ns: 350_000,
            preroll_ns: 0,
        }
    }
    #[test]
    fn cached_work_bound_includes_verified_absence() {
        let (mut index, mut source, calls) = fixture();
        let cancel = AtomicBool::new(false);
        let mut limits = SeekLimits {
            max_scan_headers: 5,
            ..Default::default()
        };
        source.begin_operation().unwrap();
        assert!(matches!(
            index.locate(&mut source, timing(2), request(), limits, &cancel),
            Err(SeekError::Limit("scan headers"))
        ));
        assert_eq!(index.cached_track, None);
        limits.max_scan_headers = 6;
        source.begin_operation().unwrap();
        let (cold, candidate) = index
            .locate(&mut source, timing(2), request(), limits, &cancel)
            .unwrap();
        assert!(candidate);
        assert_eq!(index.cached_track, None);
        let clusters: Vec<_> = index
            .entries
            .iter()
            .filter(|e| e.header.id.value() == CLUSTER)
            .collect();
        assert_eq!(clusters.len(), 3);
        assert!(clusters[1].cached_first.is_none());
        assert_eq!(
            clusters.iter().map(|e| e.cached_steps).collect::<Vec<_>>(),
            [2, 2, 2]
        );
        index.commit_fallback(2);
        source.begin_operation().unwrap();
        let before = calls.get();
        limits.max_scan_headers = 5;
        assert!(matches!(
            index.locate(&mut source, timing(2), request(), limits, &cancel),
            Err(SeekError::Limit("scan headers"))
        ));
        assert_eq!(calls.get(), before);
        assert_eq!(source.report().calls, 0);
        limits.max_scan_headers = 6;
        source.begin_operation().unwrap();
        let (cached, candidate) = index
            .locate(&mut source, timing(2), request(), limits, &cancel)
            .unwrap();
        assert!(!candidate);
        assert_eq!(cached, cold);
        assert_eq!(calls.get(), before);
    }
    #[test]
    fn failed_other_track_invalidates_before_overwriting() {
        let (mut index, mut source, calls) = fixture();
        let cancel = AtomicBool::new(false);
        let full = SeekLimits {
            max_scan_headers: 6,
            ..Default::default()
        };
        let (old, candidate) = index
            .locate(&mut source, timing(2), request(), full, &cancel)
            .unwrap();
        assert!(candidate);
        index.commit_fallback(2);
        source.begin_operation().unwrap();
        let short = SeekLimits {
            max_scan_headers: 3,
            ..Default::default()
        };
        assert!(matches!(
            index.locate(&mut source, timing(3), request(), short, &cancel),
            Err(SeekError::Limit("scan headers"))
        ));
        assert_eq!(index.cached_track, None);
        assert!(
            index
                .entries
                .iter()
                .find(|e| e.header.id.value() == CLUSTER)
                .unwrap()
                .cached_first
                .is_none()
        );
        source.begin_operation().unwrap();
        let before = calls.get();
        let (again, candidate) = index
            .locate(&mut source, timing(2), request(), full, &cancel)
            .unwrap();
        assert!(candidate);
        assert_eq!(again, old);
        assert!(calls.get() > before);
        assert_eq!(index.cached_track, None);
    }
}
