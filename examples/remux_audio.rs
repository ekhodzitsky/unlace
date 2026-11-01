//! Remux selected encoded audio to staged live or indexed WebM, without decoding.
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};
use unlace::{block::Lacing, demux::*, write::*};

enum Mux {
    Live(LiveWriter<BufWriter<File>>),
    Indexed(IndexedWriter<BufWriter<File>>),
}
impl Mux {
    fn block(&mut self, b: EncodedBlock<'_>) -> Result<(), WriteError> {
        match self {
            Self::Live(w) => w.write_block(b),
            Self::Indexed(w) => w.write_block(b),
        }
    }
    fn finish(&mut self) -> Result<WriteReceipt, WriteError> {
        match self {
            Self::Live(w) => w.finish(),
            Self::Indexed(w) => w.finish(),
        }
    }
    fn sink(self) -> BufWriter<File> {
        match self {
            Self::Live(w) => w.into_inner(),
            Self::Indexed(w) => w.into_inner(),
        }
    }
}
#[derive(Clone, Copy)]
struct Group {
    number: u64,
    time: i128,
    duration: Option<u64>,
    padding: i64,
    key: bool,
    invisible: bool,
    discardable: bool,
    offset: u64,
    count: u16,
}
impl Group {
    fn from(p: &Packet<'_>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            number: p.track_number,
            time: p.timestamp_ns.ok_or("first frame time unavailable")?,
            duration: p.block_duration_ns.map(u64::try_from).transpose()?,
            padding: p.block_discard_padding_ns,
            key: p.keyframe,
            invisible: p.invisible,
            discardable: p.discardable,
            offset: p.block_offset,
            count: p.lace_count,
        })
    }
    fn block(self, frames: Frames<'_>) -> EncodedBlock<'_> {
        EncodedBlock {
            track_number: self.number,
            timestamp_ns: self.time,
            frames,
            duration_ns: self.duration,
            discard_padding_ns: self.padding,
            keyframe: self.key,
            discardable: self.discardable,
            invisible: self.invisible,
            references: &[],
        }
    }
}
fn remux(
    input: &Path,
    sink: File,
    indexed: bool,
    selection: AudioSelection,
) -> Result<(File, WriteReceipt, Completion), Box<dyn std::error::Error>> {
    let limits = DemuxLimits::default();
    let mut reader = Demuxer::new(BufReader::new(File::open(input)?), selection, limits)?;
    let mut sink = Some(BufWriter::with_capacity(65536, sink));
    let mut mux = None;
    let mut group = None;
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut retained = 0usize;
    let source_completion = loop {
        match reader.poll()? {
            DemuxEvent::TracksReady => {
                let track = reader
                    .tracks()
                    .iter()
                    .find(|t| {
                        t.enabled
                            && t.track_type == 2
                            && match selection {
                                AudioSelection::First => true,
                                AudioSelection::Number(n) => t.number == n,
                            }
                    })
                    .ok_or("selected audio metadata missing")?;
                // Use one-nanosecond ticks explicitly so no source clock is rounded, even
                // when Info follows Tracks. This changes the unit, not presentation times.
                let config = WriteConfig {
                    timestamp_scale_ns: 1,
                    ..Default::default()
                };
                let tracks = [TrackConfig::from_audio(track)?];
                let output = sink.take().ok_or("duplicate TracksReady")?;
                mux = Some(if indexed {
                    Mux::Indexed(IndexedWriter::new(output, &tracks, config)?)
                } else {
                    Mux::Live(LiveWriter::new(output, &tracks, config)?)
                });
            }
            DemuxEvent::Packet(p) => {
                let w = mux.as_mut().ok_or("packet before TracksReady")?;
                if p.lace_count == 1 {
                    w.block(Group::from(&p)?.block(Frames::Single(p.data)))?;
                    continue;
                }
                if p.lace_index == 0 {
                    if group.is_some() {
                        return Err("unfinished laced group".into());
                    }
                    group = Some(Group::from(&p)?);
                    retained = 0;
                    frames.clear();
                    frames.try_reserve_exact(usize::from(p.lace_count))?;
                }
                let g = group.ok_or("laced continuation without group")?;
                if g.offset != p.block_offset
                    || usize::from(p.lace_index) != frames.len()
                    || g.count != p.lace_count
                {
                    return Err("laced group identity changed".into());
                }
                retained = retained
                    .checked_add(p.data.len())
                    .ok_or("laced size overflow")?;
                if retained > limits.block.max_block_size {
                    return Err("laced output copy limit".into());
                }
                let mut owned = Vec::new();
                owned.try_reserve_exact(p.data.len())?;
                owned.extend_from_slice(p.data);
                frames.push(owned);
                if p.lace_index + 1 == p.lace_count {
                    let mut views = Vec::new();
                    views.try_reserve_exact(frames.len())?;
                    for frame in &frames {
                        views.push(frame.as_slice());
                    }
                    w.block(g.block(Frames::Laced {
                        frames: &views,
                        lacing: Lacing::Ebml,
                    }))?;
                    group = None;
                    frames.clear();
                }
            }
            DemuxEvent::Yield => (),
            DemuxEvent::NeedInput => return Err("file data unavailable".into()),
            DemuxEvent::End(c) => break c,
        }
    };
    if group.is_some() {
        return Err("incomplete final laced group".into());
    }
    let mut mux = mux.ok_or("missing audio metadata")?;
    let receipt = mux.finish()?;
    let file = mux.sink().into_inner()?;
    file.sync_all()?;
    Ok((file, receipt, source_completion))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = PathBuf::from(
        args.next()
            .ok_or("usage: remux_audio INPUT OUTPUT live|indexed [TRACK_NUMBER]")?,
    );
    let output = PathBuf::from(args.next().ok_or("missing output path")?);
    let indexed = match args.next().and_then(|s| s.into_string().ok()).as_deref() {
        Some("indexed") => true,
        Some("live") => false,
        _ => return Err("mode must be live or indexed".into()),
    };
    let selection = match args.next() {
        Some(n) => AudioSelection::Number(n.to_str().ok_or("track number is not text")?.parse()?),
        None => AudioSelection::First,
    };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let mut name = output.as_os_str().to_os_string();
    name.push(".unlace-partial");
    let staging = PathBuf::from(name);
    // create_new protects existing staging data. No existing destination is replaced.
    let file = File::create_new(&staging)?;
    let result = remux(&input, file, indexed, selection);
    let (file, receipt, source_completion) = match result {
        Ok(r) => r,
        Err(error) => {
            eprintln!(
                "Remux failed; staging output retained at {}",
                staging.display()
            );
            return Err(error);
        }
    };
    drop(file);
    // A same-directory hard link publishes a complete file without overwriting an
    // existing destination. Directory durability is outside this portable example.
    fs::hard_link(&staging, &output)?;
    if let Err(error) = fs::remove_file(&staging) {
        eprintln!("Output published; staging cleanup failed: {error}");
    }
    println!(
        "input={source_completion:?} output={:?} packets={} blocks={} cues={} bytes={}",
        receipt.completion, receipt.packets, receipt.blocks, receipt.cues, receipt.end_offset
    );
    Ok(())
}
