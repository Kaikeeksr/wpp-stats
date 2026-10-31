pub mod cache;
pub mod civil;
pub mod duration;
mod emoji;
pub mod export;
pub mod index;
pub mod media;
pub mod parse;
mod patterns;
pub mod tz;
pub mod unzip;

pub use rayon::ThreadPool;

use media::{kind_of, MediaIndex};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::time::{Duration, Instant};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Texto,
    Audio,
    Foto,
    Video,
    Figurinha,
    Gif,
    Documento,
    FotoUnica,
    VideoUnico,
    AudioUnico,
    Outros,
}

impl Kind {
    pub const ALL: [Kind; 11] = [
        Kind::Texto,
        Kind::Audio,
        Kind::Foto,
        Kind::Video,
        Kind::Figurinha,
        Kind::Gif,
        Kind::Documento,
        Kind::FotoUnica,
        Kind::VideoUnico,
        Kind::AudioUnico,
        Kind::Outros,
    ];

    pub fn from_u8(k: u8) -> Kind {
        Self::ALL
            .get(usize::from(k))
            .copied()
            .unwrap_or(Kind::Outros)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Events {
    pub ts: Vec<i64>,
    pub author: Vec<u8>,
    pub kind: Vec<u8>,
    pub tbytes: Vec<u32>,
    pub tchars: Vec<u32>,
    pub msize: Vec<u64>,
    pub dur_ms: Vec<u32>,
}

impl Events {
    pub fn len(&self) -> usize {
        self.ts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ts.is_empty()
    }

    fn reserve(&mut self, n: usize) {
        self.ts.reserve(n);
        self.author.reserve(n);
        self.kind.reserve(n);
        self.tbytes.reserve(n);
        self.tchars.reserve(n);
        self.msize.reserve(n);
        self.dur_ms.reserve(n);
    }

    fn permute(&mut self, perm: &[u32]) {
        fn apply<T: Copy>(v: &mut Vec<T>, perm: &[u32]) {
            *v = perm.iter().map(|&i| v[i as usize]).collect();
        }
        apply(&mut self.ts, perm);
        apply(&mut self.author, perm);
        apply(&mut self.kind, perm);
        apply(&mut self.tbytes, perm);
        apply(&mut self.tchars, perm);
        apply(&mut self.msize, perm);
        apply(&mut self.dur_ms, perm);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateOrder {
    #[default]
    Dmy,
    Mdy,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Integrity {
    pub missing: Vec<String>,
    pub orphans: Vec<String>,
    pub no_duration: Vec<String>,
    pub ignored: Vec<u32>,
    pub invalid_lines: u32,
    pub no_author: u32,
    pub bad_dates: u32,
    pub date_order: DateOrder,
    pub date_order_guessed: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Extras {
    pub deleted: Vec<(i64, u8)>,
    pub links: Vec<(u32, u32)>,
    pub emojis: Vec<String>,
    pub emoji_uses: Vec<(u32, u32, u32)>,
    pub sticker_files: Vec<String>,
    pub stickers: Vec<(u32, u32)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Analysis {
    pub authors: Vec<String>,
    pub events: Events,
    pub captioned: Vec<u32>,
    pub extras: Extras,
    pub integrity: Integrity,
    pub chat_sha256: [u8; 32],
    pub analyzed_at: i64,
    #[serde(skip)]
    pub timings: Vec<(&'static str, Duration)>,
    #[serde(skip)]
    pub from_cache: bool,
    #[serde(skip)]
    pub extras_pending: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("a pasta não contém a conversa exportada (_chat.txt ou o .txt do WhatsApp): {0}")]
    NoChat(PathBuf),
    #[error("erro de leitura em {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("falha na exportação: {0}")]
    Export(String),
    #[error("fuso desconhecido: {0}")]
    Zone(String),
    #[error("falha ao criar o pool de threads: {0}")]
    Pool(#[from] rayon::ThreadPoolBuildError),
    #[error("análise cancelada")]
    Cancelled,
}

#[derive(Clone, Copy, Debug)]
pub enum Stage {
    Unzip,
    Listing,
    Parsing,
    Durations,
    Index,
}

const STAGE_WEIGHT: [f64; 5] = [0.0, 0.05, 0.35, 0.55, 0.05];
const STAGE_WEIGHT_ZIP: [f64; 5] = [0.7, 0.02, 0.1, 0.16, 0.02];

#[derive(Debug, Default)]
pub struct Progress {
    done: [AtomicU64; 5],
    total: [AtomicU64; 5],
    cancel: AtomicBool,
}

impl Progress {
    pub fn start(&self, s: Stage, total: u64) {
        self.done[s as usize].store(0, Relaxed);
        self.total[s as usize].store(total.max(1), Relaxed);
    }

    pub fn add(&self, s: Stage, n: u64) {
        self.done[s as usize].fetch_add(n, Relaxed);
    }

    pub fn finish(&self, s: Stage) {
        self.total[s as usize].store(1, Relaxed);
        self.done[s as usize].store(1, Relaxed);
    }

    pub fn fraction(&self) -> f64 {
        let w = if self.total[Stage::Unzip as usize].load(Relaxed) == 0 {
            &STAGE_WEIGHT
        } else {
            &STAGE_WEIGHT_ZIP
        };
        (0..5)
            .map(|i| {
                let t = self.total[i].load(Relaxed);
                let d = self.done[i].load(Relaxed).min(t);
                if t == 0 {
                    0.0
                } else {
                    w[i] * d as f64 / t as f64
                }
            })
            .sum()
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Relaxed);
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Relaxed)
    }

    fn check(&self) -> Result<(), Error> {
        if self.cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}

const MMAP_THRESHOLD: u64 = 512 << 20;

pub fn new_pool() -> Result<rayon::ThreadPool, Error> {
    let n = std::thread::available_parallelism().map_or(1, |n| n.get());
    Ok(rayon::ThreadPoolBuilder::new()
        .num_threads(n.saturating_sub(1).max(1))
        .build()?)
}

pub fn analyze(dir: &Path, pool: &rayon::ThreadPool) -> Result<Analysis, Error> {
    open(dir, pool, None)
}

pub fn open(dir: &Path, pool: &rayon::ThreadPool, cache: Option<&Path>) -> Result<Analysis, Error> {
    open_with(dir, pool, cache, &Progress::default(), false)
}

pub fn app_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("WppStats"))
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub(crate) fn io(path: &Path) -> impl FnOnce(std::io::Error) -> Error {
    let path = path.to_path_buf();
    move |source| Error::Io { path, source }
}

pub(crate) fn mtime_ns(md: &std::fs::Metadata) -> u64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos() as u64)
}

fn read_chat(dir: &Path, media: &MediaIndex) -> Result<Box<dyn AsRef<[u8]> + Send + Sync>, Error> {
    let chat_name = media
        .chat
        .as_deref()
        .ok_or_else(|| Error::NoChat(dir.to_path_buf()))?;
    let chat_path = dir.join(chat_name);
    let file = std::fs::File::open(&chat_path).map_err(|_| Error::NoChat(dir.to_path_buf()))?;
    let len = file.metadata().map_err(io(&chat_path))?.len();
    Ok(if len > MMAP_THRESHOLD {
        Box::new(unsafe { memmap2::Mmap::map(&file) }.map_err(io(&chat_path))?)
    } else {
        Box::new(std::fs::read(&chat_path).map_err(io(&chat_path))?)
    })
}

pub fn open_with(
    dir: &Path,
    pool: &rayon::ThreadPool,
    cache: Option<&Path>,
    progress: &Progress,
    defer_extras: bool,
) -> Result<Analysis, Error> {
    let mut timings = Vec::new();
    let mut t = Instant::now();
    let mut lap = |name, timings: &mut Vec<_>| {
        timings.push((name, t.elapsed()));
        t = Instant::now();
    };

    progress.start(Stage::Listing, 1);
    let media = MediaIndex::list(dir).map_err(io(dir))?;
    progress.finish(Stage::Listing);
    lap("listagem", &mut timings);
    progress.check()?;
    if let Some(c) = cache {
        if let Some(mut a) = cache::load(c, dir, media.fingerprint) {
            progress.finish(Stage::Parsing);
            progress.finish(Stage::Durations);
            lap("cache", &mut timings);
            a.timings = timings;
            a.from_cache = true;
            return Ok(a);
        }
    }

    let chat = read_chat(dir, &media)?;
    let buf: &[u8] = (*chat).as_ref();
    lap("leitura", &mut timings);

    progress.start(Stage::Parsing, buf.len() as u64);
    let (sha, mut parsed) = std::thread::scope(|s| {
        let sha = s.spawn(|| Sha256::digest(buf));
        let parsed = parse::parse_with(buf, &media, pool, !defer_extras);
        (sha.join().expect("sha256 thread"), parsed)
    });
    progress.finish(Stage::Parsing);
    lap("parsing", &mut timings);
    progress.check()?;

    let used = used(&media, &parsed.refs);
    parsed.integrity.orphans = media
        .entries
        .iter()
        .zip(&used)
        .filter(|(_, &u)| !u)
        .map(|(e, _)| e.name.clone())
        .collect();
    lap("resolução", &mut timings);

    let timed = of_kind(&media, &used, Kind::Audio, Kind::Video);
    progress.start(Stage::Durations, timed.len() as u64);
    let durs: Vec<Option<u32>> = pool.install(|| {
        timed
            .par_iter()
            .map(|&e| {
                if progress.cancelled() {
                    return None;
                }
                let d = duration::duration_ms(&dir.join(&media.entries[e as usize].name));
                progress.add(Stage::Durations, 1);
                d
            })
            .collect()
    });
    progress.check()?;
    let mut dur_of = vec![None; media.entries.len()];
    for (&e, d) in timed.iter().zip(durs) {
        dur_of[e as usize] = d;
        if d.is_none() {
            parsed
                .integrity
                .no_duration
                .push(media.entries[e as usize].name.clone());
        }
    }
    for &(ev, e) in &parsed.refs {
        if let Some(d) = dur_of[e as usize] {
            parsed.events.dur_ms[ev as usize] = d;
        }
    }
    lap("durações", &mut timings);

    if !defer_extras {
        (parsed.extras.sticker_files, parsed.extras.stickers) =
            group_stickers(dir, &media, &used, &parsed.refs, pool);
        lap("figurinhas", &mut timings);
    }

    let mut a = Analysis {
        authors: parsed.authors,
        events: parsed.events,
        captioned: parsed.captioned,
        extras: parsed.extras,
        integrity: parsed.integrity,
        chat_sha256: sha.into(),
        analyzed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64),
        timings: Vec::new(),
        from_cache: false,
        extras_pending: defer_extras,
    };
    if let (Some(c), false) = (cache, defer_extras) {
        let _ = cache::store(c, dir, media.fingerprint, &a);
        lap("gravação do cache", &mut timings);
    }
    a.timings = timings;
    Ok(a)
}

fn used(media: &MediaIndex, refs: &[(u32, u32)]) -> Vec<bool> {
    let mut used = vec![false; media.entries.len()];
    for &(_, e) in refs {
        used[e as usize] = true;
    }
    used
}

fn of_kind(media: &MediaIndex, used: &[bool], a: Kind, b: Kind) -> Vec<u32> {
    let mut v: Vec<u32> = (0..media.entries.len() as u32)
        .filter(|&e| used[e as usize] && [a, b].contains(&kind_of(&media.entries[e as usize].name)))
        .collect();
    v.sort_by(|&x, &y| {
        media.entries[x as usize]
            .name
            .cmp(&media.entries[y as usize].name)
    });
    v
}

fn group_stickers(
    dir: &Path,
    media: &MediaIndex,
    used: &[bool],
    refs: &[(u32, u32)],
    pool: &rayon::ThreadPool,
) -> (Vec<String>, Vec<(u32, u32)>) {
    let stickers = of_kind(media, used, Kind::Figurinha, Kind::Figurinha);
    let size = |e: u32| media.entries[e as usize].size;
    let mut same_size: ahash::AHashMap<u64, u32> = Default::default();
    for &e in &stickers {
        *same_size.entry(size(e)).or_default() += 1;
    }
    let hashes: Vec<Option<[u8; 32]>> = pool.install(|| {
        stickers
            .par_iter()
            .map(|&e| {
                (same_size[&size(e)] > 1)
                    .then(|| std::fs::read(dir.join(&media.entries[e as usize].name)).ok())
                    .flatten()
                    .map(|b| Sha256::digest(b).into())
            })
            .collect()
    });
    let mut files = Vec::new();
    let mut group_of = vec![u32::MAX; media.entries.len()];
    let mut groups: ahash::AHashMap<[u8; 32], u32> = Default::default();
    for (&e, h) in stickers.iter().zip(hashes) {
        let next = files.len() as u32;
        let id = match h {
            Some(h) => *groups.entry(h).or_insert(next),
            None => next,
        };
        if id == next {
            files.push(media.entries[e as usize].name.clone());
        }
        group_of[e as usize] = id;
    }
    let mut uses: Vec<(u32, u32)> = refs
        .iter()
        .filter(|&&(_, e)| group_of[e as usize] != u32::MAX)
        .map(|&(ev, e)| (ev, group_of[e as usize]))
        .collect();
    uses.sort_unstable();
    (files, uses)
}

pub struct Deferred {
    chat_sha256: [u8; 32],
    fingerprint: media::Fingerprint,
    emojis: Vec<String>,
    emoji_uses: Vec<(u32, u32, u32)>,
    sticker_files: Vec<String>,
    stickers: Vec<(u32, u32)>,
}

pub fn deferred_extras(dir: &Path, pool: &rayon::ThreadPool) -> Result<Deferred, Error> {
    let media = MediaIndex::list(dir).map_err(io(dir))?;
    let chat = read_chat(dir, &media)?;
    let buf: &[u8] = (*chat).as_ref();
    let parsed = parse::parse(buf, &media, pool);
    let used = used(&media, &parsed.refs);
    let (sticker_files, stickers) = group_stickers(dir, &media, &used, &parsed.refs, pool);
    Ok(Deferred {
        chat_sha256: Sha256::digest(buf).into(),
        fingerprint: media.fingerprint,
        emojis: parsed.extras.emojis,
        emoji_uses: parsed.extras.emoji_uses,
        sticker_files,
        stickers,
    })
}

impl Analysis {
    pub fn complete(&mut self, d: Deferred, dir: &Path, cache: Option<&Path>) {
        self.extras_pending = false;
        if d.chat_sha256 != self.chat_sha256 {
            return;
        }
        self.extras.emojis = d.emojis;
        self.extras.emoji_uses = d.emoji_uses;
        self.extras.sticker_files = d.sticker_files;
        self.extras.stickers = d.stickers;
        if let Some(c) = cache {
            let _ = cache::store(c, dir, d.fingerprint, self);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_weights_and_cancel() {
        let p = Progress::default();
        assert_eq!(p.fraction(), 0.0);
        p.finish(Stage::Listing);
        p.start(Stage::Durations, 4);
        p.add(Stage::Durations, 2);
        assert!((p.fraction() - (0.05 + 0.55 / 2.0)).abs() < 1e-9);
        for s in [
            Stage::Unzip,
            Stage::Listing,
            Stage::Parsing,
            Stage::Durations,
            Stage::Index,
        ] {
            p.finish(s);
        }
        assert!((p.fraction() - 1.0).abs() < 1e-9);
        assert!(p.check().is_ok());
        p.cancel();
        assert!(matches!(p.check(), Err(Error::Cancelled)));
    }
}
