use crate::civil::days_from_civil;
use crate::media::{kind_of, MediaIndex};
use crate::patterns::{Pat, LRM, PATTERNS};
use crate::{emoji, DateOrder, Events, Extras, Integrity, Kind};
use ahash::AHashMap;
use memchr::memmem;
use rayon::prelude::*;

const PAR_THRESHOLD: usize = 64 << 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Header {
    pub f1: u8,
    pub f2: u8,
    pub year: u16,
    pub secs: u32,
}

#[derive(Debug, Default)]
pub struct Parsed {
    pub authors: Vec<String>,
    pub events: Events,
    pub captioned: Vec<u32>,
    pub refs: Vec<(u32, u32)>,
    pub extras: Extras,
    pub integrity: Integrity,
}

pub fn parse(buf: &[u8], media: &MediaIndex, pool: &rayon::ThreadPool) -> Parsed {
    parse_with(buf, media, pool, true)
}

pub fn parse_with(
    buf: &[u8],
    media: &MediaIndex,
    pool: &rayon::ThreadPool,
    emojis: bool,
) -> Parsed {
    let crlf = memmem::find(buf, b"\r\n").is_some();
    let mut part = if buf.len() > PAR_THRESHOLD {
        let chunks = split(buf, crlf, pool.current_num_threads().max(1) * 2);
        let parts: Vec<Part> = pool.install(|| {
            chunks
                .par_iter()
                .map(|&(s, e)| parse_part(&buf[s..e], crlf, media, emojis))
                .collect()
        });
        merge(parts)
    } else {
        parse_part(buf, crlf, media, emojis)
    };

    let (order, guessed) = detect_order(&part.hdr);
    let mut bad_dates = 0;
    part.ev.ts = part
        .hdr
        .iter()
        .map(|h| {
            let (ts, ok) = timestamp(*h, order);
            bad_dates += u32::from(!ok);
            ts
        })
        .collect();

    let mut deleted: Vec<(i64, u8)> = part
        .deleted
        .iter()
        .map(|&(h, a)| (timestamp(h, order).0, a))
        .collect();
    deleted.sort_by_key(|d| d.0);

    let mut p = Parsed {
        authors: part
            .authors
            .iter()
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect(),
        events: part.ev,
        captioned: part.captioned,
        refs: part.refs,
        extras: Extras {
            deleted,
            links: part.links,
            emojis: part.emojis,
            emoji_uses: part.emoji_uses,
            ..Default::default()
        },
        integrity: Integrity {
            missing: part.missing,
            ignored: part.ignored,
            invalid_lines: part.invalid_lines,
            no_author: part.no_author,
            bad_dates,
            date_order: order,
            date_order_guessed: guessed,
            ..Default::default()
        },
    };
    sort_if_needed(&mut p);
    p
}

fn split(buf: &[u8], crlf: bool, n: usize) -> Vec<(usize, usize)> {
    let sep: &[u8] = if crlf { b"\r\n" } else { b"\n" };
    let finder = memmem::Finder::new(sep);
    let mut out = Vec::with_capacity(n);
    let mut start = 0;
    for k in 1..n {
        let mut at = (buf.len() * k / n).max(start);
        let cut = loop {
            match finder.find(&buf[at..]) {
                Some(i) => {
                    let line = at + i + sep.len();
                    if parse_header(&buf[line..]).is_some() {
                        break line;
                    }
                    at = line;
                }
                None => break buf.len(),
            }
        };
        if cut > start && cut < buf.len() {
            out.push((start, cut));
            start = cut;
        }
    }
    out.push((start, buf.len()));
    out
}

#[derive(Default)]
struct Part {
    authors: Vec<Vec<u8>>,
    hdr: Vec<Header>,
    ev: Events,
    ignored: Vec<u32>,
    refs: Vec<(u32, u32)>,
    captioned: Vec<u32>,
    missing: Vec<String>,
    invalid_lines: u32,
    no_author: u32,
    deleted: Vec<(Header, u8)>,
    links: Vec<(u32, u32)>,
    emojis: Vec<String>,
    emoji_ids: AHashMap<String, u32>,
    emoji_uses: Vec<(u32, u32, u32)>,
    scan_emojis: bool,
}

struct Msg {
    hdr: Header,
    start: usize,
    end: usize,
    joined: bool,
}

fn parse_part(buf: &[u8], crlf: bool, media: &MediaIndex, scan_emojis: bool) -> Part {
    let sep: &[u8] = if crlf { b"\r\n" } else { b"\n" };
    let mut p = Part {
        scan_emojis,
        ..Default::default()
    };
    let mut cur: Option<Msg> = None;
    let mut start = 0;
    let ends = memmem::find_iter(buf, sep)
        .map(|e| (e, true))
        .chain(std::iter::once((buf.len(), false)));
    for (end, terminated) in ends {
        let raw = &buf[start..end];
        let line = if crlf {
            raw
        } else {
            raw.strip_suffix(b"\r").unwrap_or(raw)
        };
        if !terminated && line.is_empty() {
            break;
        }
        match parse_header(line) {
            Some((hdr, off)) => {
                if let Some(m) = cur.take() {
                    p.message(buf, m, media);
                }
                cur = Some(Msg {
                    hdr,
                    start: start + off,
                    end: start + line.len(),
                    joined: false,
                });
            }
            None if !terminated && looks_like_header(line) => p.invalid_lines += 1,
            None => match cur.as_mut() {
                Some(m) => {
                    m.end = start + line.len();
                    m.joined = true;
                }
                None => p.invalid_lines += u32::from(!line.is_empty()),
            },
        }
        start = end + sep.len();
    }
    if let Some(m) = cur {
        p.message(buf, m, media);
    }
    p
}

impl Part {
    fn intern(&mut self, name: &[u8]) -> u8 {
        if let Some(i) = self.authors.iter().position(|a| a == name) {
            return i as u8;
        }
        if self.authors.len() == 256 {
            return 255;
        }
        self.authors.push(name.to_vec());
        self.ignored.push(0);
        (self.authors.len() - 1) as u8
    }

    fn intern_emoji(&mut self, e: &str) -> u32 {
        if let Some(&id) = self.emoji_ids.get(e) {
            return id;
        }
        let id = self.emojis.len() as u32;
        self.emojis.push(e.into());
        self.emoji_ids.insert(e.into(), id);
        id
    }

    fn measure(&mut self, ev: u32, text: &[u8]) {
        let links = PATTERNS.count_links(text);
        if links > 0 {
            self.links.push((ev, links));
        }
        if !self.scan_emojis || !text.iter().any(|&b| b >= 0xE2) {
            return;
        }
        emoji::scan(&String::from_utf8_lossy(text), |e| {
            let id = self.intern_emoji(e);
            match self.emoji_uses.last_mut() {
                Some(u) if u.0 == ev && u.1 == id => u.2 += 1,
                _ => self.emoji_uses.push((ev, id, 1)),
            }
        });
    }

    fn push(&mut self, hdr: Header, author: u8, kind: Kind, text: &[u8], joined: bool, msize: u64) {
        let crlfs = if joined {
            memmem::find_iter(text, b"\r\n").count() as u32
        } else {
            0
        };
        self.hdr.push(hdr);
        let e = &mut self.ev;
        e.ts.push(0);
        e.author.push(author);
        e.kind.push(kind as u8);
        e.tbytes.push(text.len() as u32 - crlfs);
        e.tchars.push(bytecount::num_chars(text) as u32 - crlfs);
        e.msize.push(msize);
        e.dur_ms.push(0);
        if !text.is_empty() {
            self.measure(self.ev.len() as u32 - 1, text);
        }
    }

    fn message(&mut self, buf: &[u8], m: Msg, media: &MediaIndex) {
        let rest = &buf[m.start..m.end];
        let colon = memchr::memchr_iter(b':', rest).find(|&i| rest.get(i + 1) == Some(&b' '));
        let Some(colon) = colon else {
            self.no_author += 1;
            return;
        };
        let a = self.intern(trim(&rest[..colon]));
        let mut body = &rest[colon + 2..];
        let marked = body.starts_with(LRM);
        if marked {
            body = &body[LRM.len()..];
        }
        let (h, j) = (m.hdr, m.joined);

        let nl = memchr::memchr(b'\n', body).unwrap_or(body.len());
        let kind = match PATTERNS.classify(&body[..nl]) {
            Some(Pat::System) => {
                self.ignored[a as usize] += 1;
                return;
            }
            Some(Pat::Deleted) => {
                self.ignored[a as usize] += 1;
                self.deleted.push((h, a));
                return;
            }
            Some(Pat::FotoUnica) => Some(Kind::FotoUnica),
            Some(Pat::VideoUnico) => Some(Kind::VideoUnico),
            Some(Pat::AudioUnico) => Some(Kind::AudioUnico),
            Some(Pat::Other) => Some(Kind::Outros),
            None => None,
        };
        if let Some(kind) = kind {
            let ev = self.ev.len() as u32;
            self.push(h, a, kind, b"", j, 0);
            let caption = &body[(nl + 1).min(body.len())..];
            if !trim(caption).is_empty() {
                self.captioned.push(ev);
                self.push(h, a, Kind::Texto, caption, j, 0);
            }
            return;
        }

        let attach = PATTERNS
            .find_attach(body)
            .map(|(at, name_start)| {
                let tail = &body[name_start..];
                let name = &tail[..memchr::memchr(b'>', tail).unwrap_or(tail.len())];
                (name, &body[..at])
            })
            .or_else(|| {
                PATTERNS
                    .find_attached(body)
                    .map(|(name_end, cap)| (&body[..name_end], &body[cap..]))
            });
        if let Some((name, caption)) = attach {
            let name = String::from_utf8_lossy(name);
            let ev = self.ev.len() as u32;
            let size = match media.get(&name) {
                Some(e) => {
                    self.refs.push((ev, e));
                    media.entries[e as usize].size
                }
                None => {
                    self.missing.push(name.to_string());
                    0
                }
            };
            self.push(h, a, kind_of(&name), b"", j, size);
            if !trim(caption).is_empty() {
                self.captioned.push(ev);
                self.push(h, a, Kind::Texto, caption, j, 0);
            }
            return;
        }

        if marked {
            return self.push(h, a, Kind::Outros, b"", j, 0);
        }
        let body = PATTERNS.strip_edited(body).unwrap_or(body);
        self.push(h, a, Kind::Texto, body, j, 0);
    }
}

fn merge(mut parts: Vec<Part>) -> Part {
    let mut base = parts.remove(0);
    let more: usize = parts.iter().map(|p| p.ev.len()).sum();
    base.ev.reserve(more);
    base.hdr.reserve(more);
    for p in parts {
        let off = base.ev.len() as u32;
        let remap: Vec<u8> = p.authors.iter().map(|a| base.intern(a)).collect();
        for (i, n) in p.ignored.iter().enumerate() {
            base.ignored[remap[i] as usize] += n;
        }
        let e = p.ev;
        base.ev
            .author
            .extend(e.author.iter().map(|&a| remap[a as usize]));
        base.ev.ts.extend(e.ts);
        base.ev.kind.extend(e.kind);
        base.ev.tbytes.extend(e.tbytes);
        base.ev.tchars.extend(e.tchars);
        base.ev.msize.extend(e.msize);
        base.ev.dur_ms.extend(e.dur_ms);
        base.hdr.extend(p.hdr);
        base.refs
            .extend(p.refs.iter().map(|&(ev, m)| (ev + off, m)));
        base.captioned
            .extend(p.captioned.iter().map(|&ev| ev + off));
        base.missing.extend(p.missing);
        base.deleted
            .extend(p.deleted.iter().map(|&(h, a)| (h, remap[a as usize])));
        base.links
            .extend(p.links.iter().map(|&(ev, n)| (ev + off, n)));
        let ids: Vec<u32> = p.emojis.iter().map(|e| base.intern_emoji(e)).collect();
        base.emoji_uses.extend(
            p.emoji_uses
                .iter()
                .map(|&(ev, id, n)| (ev + off, ids[id as usize], n)),
        );
        base.invalid_lines += p.invalid_lines;
        base.no_author += p.no_author;
    }
    base
}

fn sort_if_needed(p: &mut Parsed) {
    let ts = &p.events.ts;
    if ts.windows(2).all(|w| w[0] <= w[1]) {
        return;
    }
    let mut perm: Vec<u32> = (0..ts.len() as u32).collect();
    perm.sort_by_key(|&i| ts[i as usize]);
    p.events.permute(&perm);
    let mut new_pos = vec![0u32; perm.len()];
    for (new, &old) in perm.iter().enumerate() {
        new_pos[old as usize] = new as u32;
    }
    for r in &mut p.refs {
        r.0 = new_pos[r.0 as usize];
    }
    for c in &mut p.captioned {
        *c = new_pos[*c as usize];
    }
    p.captioned.sort_unstable();
    for l in &mut p.extras.links {
        l.0 = new_pos[l.0 as usize];
    }
    p.extras.links.sort_unstable();
    for u in &mut p.extras.emoji_uses {
        u.0 = new_pos[u.0 as usize];
    }
    p.extras.emoji_uses.sort_unstable();
}

fn trim(mut b: &[u8]) -> &[u8] {
    loop {
        let t = b.trim_ascii();
        let t = t.strip_prefix(LRM).unwrap_or(t);
        let t = t.strip_suffix(LRM).unwrap_or(t);
        if t.len() == b.len() {
            return t;
        }
        b = t;
    }
}

fn looks_like_header(line: &[u8]) -> bool {
    let l = line.strip_prefix(LRM).unwrap_or(line);
    let l = l.strip_prefix(b"[").unwrap_or(l);
    l.first().is_some_and(u8::is_ascii_digit) && l[..l.len().min(3)].contains(&b'/')
}

pub(crate) fn parse_header(line: &[u8]) -> Option<(Header, usize)> {
    let mut i = if line.starts_with(LRM) { LRM.len() } else { 0 };
    let bracket = eat(line, &mut i, b'[').is_some();
    let (f1, _) = digits(line, &mut i, 2)?;
    eat(line, &mut i, b'/')?;
    let (f2, _) = digits(line, &mut i, 2)?;
    eat(line, &mut i, b'/')?;
    let year = match digits(line, &mut i, 4)? {
        (y, 2) => 2000 + y,
        (y, 4) => y,
        _ => return None,
    };
    if bracket {
        eat(line, &mut i, b',')?;
    } else {
        let _ = eat(line, &mut i, b',');
    }
    eat(line, &mut i, b' ')?;
    let (mut h, _) = digits(line, &mut i, 2)?;
    eat(line, &mut i, b':')?;
    let (m, 2) = digits(line, &mut i, 2)? else {
        return None;
    };
    let s = if line.get(i) == Some(&b':') {
        i += 1;
        match digits(line, &mut i, 2)? {
            (s, 2) => s,
            _ => return None,
        }
    } else {
        0
    };
    if let Some(pm) = ampm(line, &mut i) {
        if !(1..=12).contains(&h) {
            return None;
        }
        h = h % 12 + if pm { 12 } else { 0 };
    }
    if bracket {
        eat(line, &mut i, b']')?;
        eat(line, &mut i, b' ')?;
    } else {
        line.get(i..i + 3).filter(|s| *s == b" - ")?;
        i += 3;
    }
    if f1 == 0 || f2 == 0 || h > 23 || m > 59 || s > 59 {
        return None;
    }
    let hdr = Header {
        f1: f1 as u8,
        f2: f2 as u8,
        year: year as u16,
        secs: h * 3600 + m * 60 + s,
    };
    Some((hdr, i))
}

fn eat(b: &[u8], i: &mut usize, c: u8) -> Option<()> {
    (b.get(*i) == Some(&c)).then(|| *i += 1)
}

fn digits(b: &[u8], i: &mut usize, max: usize) -> Option<(u32, usize)> {
    let start = *i;
    let mut v = 0;
    while *i - start < max {
        match b.get(*i) {
            Some(c) if c.is_ascii_digit() => v = v * 10 + u32::from(c - b'0'),
            _ => break,
        }
        *i += 1;
    }
    (*i > start).then_some((v, *i - start))
}

fn ampm(b: &[u8], i: &mut usize) -> Option<bool> {
    let mut j = *i;
    for sp in [" ", "\u{202F}", "\u{A0}"] {
        if b[j..].starts_with(sp.as_bytes()) {
            j += sp.len();
            break;
        }
    }
    let pm = match b.get(j)? {
        b'a' | b'A' => false,
        b'p' | b'P' => true,
        _ => return None,
    };
    j += 1;
    if b.get(j) == Some(&b'.') {
        j += 1;
        if b.get(j) == Some(&b' ') {
            j += 1;
        }
    }
    if !matches!(b.get(j)?, b'm' | b'M') {
        return None;
    }
    j += 1;
    if b.get(j) == Some(&b'.') {
        j += 1;
    }
    *i = j;
    Some(pm)
}

fn detect_order(h: &[Header]) -> (DateOrder, bool) {
    let f1_big = h.iter().any(|h| h.f1 > 12);
    let f2_big = h.iter().any(|h| h.f2 > 12);
    match (f1_big, f2_big) {
        (true, false) => return (DateOrder::Dmy, false),
        (false, true) => return (DateOrder::Mdy, false),
        (true, true) => return (DateOrder::Dmy, true),
        (false, false) => {}
    }
    let violations = |o| {
        h.windows(2)
            .filter(|w| timestamp(w[1], o).0 < timestamp(w[0], o).0)
            .count()
    };
    let (dmy, mdy) = (violations(DateOrder::Dmy), violations(DateOrder::Mdy));
    match dmy.cmp(&mdy) {
        std::cmp::Ordering::Less => (DateOrder::Dmy, false),
        std::cmp::Ordering::Greater => (DateOrder::Mdy, false),
        std::cmp::Ordering::Equal => (DateOrder::Dmy, true),
    }
}

fn timestamp(h: Header, order: DateOrder) -> (i64, bool) {
    let (d, m) = match order {
        DateOrder::Dmy => (h.f1, h.f2),
        DateOrder::Mdy => (h.f2, h.f1),
    };
    let (y, m, d) = (i64::from(h.year), i64::from(m), i64::from(d));
    let ok = (1..=12).contains(&m) && d <= days_in_month(y, m);
    (days_from_civil(y, m, d) * 86_400 + i64::from(h.secs), ok)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Entry;

    fn run(chat: &str) -> Parsed {
        run_with(chat.as_bytes(), &MediaIndex::default())
    }

    fn run_with(chat: &[u8], media: &MediaIndex) -> Parsed {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        parse(chat, media, &pool)
    }

    fn kinds(p: &Parsed) -> Vec<Kind> {
        p.events.kind.iter().map(|&k| Kind::from_u8(k)).collect()
    }

    const L: &str = "\u{200E}";

    #[test]
    fn normal_header() {
        let p = run("[06/01/2026, 09:20:31] Ana: oi\r\n");
        assert_eq!(p.authors, ["Ana"]);
        assert_eq!(kinds(&p), [Kind::Texto]);
        assert_eq!(p.events.tbytes, [2]);
        assert_eq!(
            p.events.ts,
            [days_from_civil(2026, 1, 6) * 86_400 + 9 * 3600 + 20 * 60 + 31]
        );
    }

    #[test]
    fn author_with_space_before_colon() {
        let p = run("[06/01/2026, 09:20:31] Ana : a\r\n[06/01/2026, 09:20:32] Ana: b\r\n");
        assert_eq!(p.authors, ["Ana"]);
        assert_eq!(p.events.author, [0, 0]);
    }

    #[test]
    fn multiline_message() {
        let p = run(
            "[06/01/2026, 09:20:31] Ana: linha 1\nlinha 2\r\n[06/01/2026, 09:20:32] Bia: x\r\n",
        );
        assert_eq!(p.events.len(), 2);
        assert_eq!(p.events.tbytes[0], 15);
        assert_eq!(p.authors, ["Ana", "Bia"]);
    }

    #[test]
    fn lrm_at_line_and_body_start() {
        let chat = format!("{L}[06/01/2026, 09:20:31] Ana: {L}imagem ocultada\r\n");
        assert_eq!(kinds(&run(&chat)), [Kind::FotoUnica]);
    }

    #[test]
    fn media_without_and_with_caption() {
        let media = MediaIndex::from_entries(vec![Entry {
            name: "00000002-PHOTO-2026-01-06-09-20-32.jpg".into(),
            size: 1000,
        }]);
        let chat = format!(
            "{L}[06/01/2026, 09:20:31] Ana: {L}<anexado: 00000001-AUDIO-2026-01-06-09-20-31.opus>\r\n\
             {L}[06/01/2026, 09:20:32] Ana: olha só {L}<anexado: 00000002-PHOTO-2026-01-06-09-20-32.jpg>\r\n"
        );
        let p = run_with(chat.as_bytes(), &media);
        assert_eq!(kinds(&p), [Kind::Audio, Kind::Foto, Kind::Texto]);
        assert_eq!(p.events.msize, [0, 1000, 0]);
        assert_eq!(p.captioned, [1]);
        assert_eq!(p.refs, [(1, 0)]);
        assert_eq!(
            p.integrity.missing,
            ["00000001-AUDIO-2026-01-06-09-20-31.opus"]
        );
        assert_eq!(p.events.tbytes[2], 12);
        assert_eq!(p.events.tchars[2], 9);
        assert_eq!(p.events.ts[1], p.events.ts[2]);
    }

    #[test]
    fn android_header_and_attachments() {
        let media = MediaIndex::from_entries(vec![Entry {
            name: "IMG-20250616-WA0020.jpg".into(),
            size: 500,
        }]);
        let chat = "3/24/25, 18:28 - Messages and calls are end-to-end encrypted. Only people in this chat can read.\n\
                    3/24/25, 18:29 - Ana: oi\n\
                    3/24/25, 18:30 - Ana: IMG-20250616-WA0020.jpg (file attached)\n\
                    olha\n\
                    3/25/25, 9:05 - Bia: PTT-20250325-WA0001.opus (file attached)\n\
                    3/25/25, 9:06 - Bia: <Media omitted>\n\
                    legenda\n\
                    3/25/25, 9:07 - Bia: <View once voice message omitted>\n\
                    3/25/25, 9:08 - Bia: You deleted this message\n\
                    3/25/25, 9:09 - Bia: location: https://maps.example/x\n\
                    3/25/25, 9:10 - Ana: certo <This message was edited>\n";
        let p = run_with(chat.as_bytes(), &media);
        assert_eq!(p.integrity.no_author, 1);
        assert_eq!(p.integrity.date_order, DateOrder::Mdy);
        assert_eq!(
            kinds(&p),
            [
                Kind::Texto,
                Kind::Foto,
                Kind::Texto,
                Kind::Audio,
                Kind::Outros,
                Kind::Texto,
                Kind::AudioUnico,
                Kind::Outros,
                Kind::Texto,
            ]
        );
        assert_eq!(p.captioned, [1, 4]);
        assert_eq!(p.events.msize[1], 500);
        assert_eq!((p.events.tbytes[2], p.events.tbytes[5]), (4, 7));
        assert_eq!(p.integrity.missing, ["PTT-20250325-WA0001.opus"]);
        assert_eq!(p.integrity.ignored, [0, 1]);
        assert_eq!(p.events.tbytes[8], 5);
        assert_eq!(
            p.events.ts[3],
            days_from_civil(2025, 3, 25) * 86_400 + 9 * 3600 + 5 * 60
        );
    }

    #[test]
    fn android_pt_header_without_comma() {
        let p = run("24/03/2025 18:28 - Ana: oi\n25/03/2025 09:05 - Bia: tudo\n");
        assert_eq!(p.authors, ["Ana", "Bia"]);
        assert_eq!(p.integrity.date_order, DateOrder::Dmy);
    }

    #[test]
    fn document_with_text_before_marker() {
        let chat = format!(
            "{L}[06/01/2026, 09:20:31] Ana: cv.pdf • 2 páginas {L}<anexado: 00000531-cv.pdf>\r\n"
        );
        let p = run(&chat);
        assert_eq!(kinds(&p), [Kind::Documento, Kind::Texto]);
        assert_eq!(p.integrity.missing, ["00000531-cv.pdf"]);
    }

    #[test]
    fn view_once() {
        let chat = format!(
            "{L}[06/01/2026, 09:20:31] Ana: {L}imagem ocultada\r\n{L}[06/01/2026, 09:20:32] Ana: {L}vídeo omitido\r\n\
             [06/01/2026, 09:20:33] Ana: vídeo omitido de novo\r\n\
             {L}[06/01/2026, 09:20:34] Ana: {L}áudio ocultado\r\n{L}[06/01/2026, 09:20:35] Ana: {L}áudio omitido\r\n"
        );
        assert_eq!(
            kinds(&run(&chat)),
            [
                Kind::FotoUnica,
                Kind::VideoUnico,
                Kind::Texto,
                Kind::AudioUnico,
                Kind::AudioUnico
            ]
        );
    }

    #[test]
    fn system_and_deleted_are_ignored() {
        let chat = format!(
            "[06/01/2026, 09:20:31] Ana: As mensagens e ligações são protegidas com a criptografia de ponta a ponta. Bla.\r\n\
             [06/01/2026, 09:20:32] Bia: {L}Mensagem apagada\r\n"
        );
        let p = run(&chat);
        assert_eq!(p.events.len(), 0);
        assert_eq!(p.integrity.ignored, [1, 1]);
        assert_eq!(
            p.extras.deleted,
            [(
                days_from_civil(2026, 1, 6) * 86_400 + 9 * 3600 + 20 * 60 + 32,
                1
            )]
        );
    }

    #[test]
    fn links_and_emojis_of_text_and_captions() {
        let chat = format!(
            "[06/01/2026, 09:20:31] Ana: olha www.a.com 😂😂 https://b.com\r\n\
             [06/01/2026, 09:20:32] Bia: ok\r\n\
             [06/01/2026, 09:20:33] Bia: 👍🏽 {L}<anexado: 00000003-PHOTO-2026-01-06-09-20-33.jpg>\r\n\
             [06/01/2026, 09:20:34] Ana: ❤️ e 😂 {L}<Mensagem editada>\r\n"
        );
        let p = run(&chat);
        let x = &p.extras;
        assert_eq!(x.links, [(0, 2)]);
        assert_eq!(x.emojis, ["😂", "👍🏽", "❤️"]);
        assert_eq!(x.emoji_uses, [(0, 0, 2), (3, 1, 1), (4, 2, 1), (4, 0, 1)]);
    }

    #[test]
    fn edited_suffix_removed() {
        let p = run(&format!(
            "[06/01/2026, 09:20:31] Ana: abc {L}<Mensagem editada>\r\n"
        ));
        assert_eq!(kinds(&p), [Kind::Texto]);
        assert_eq!(p.events.tbytes, [3]);
    }

    #[test]
    fn english_variants() {
        let chat = format!(
            "[1/6/26, 9:20:31 AM] Ann: Messages and calls are end-to-end encrypted. Bla.\r\n\
             [1/6/26, 9:20:32 AM] Ann: {L}This message was deleted\r\n\
             [1/6/26, 9:20:33 AM] Ann: {L}image omitted\r\n\
             [1/6/26, 9:20:34 AM] Ann: {L}video omitted\r\n\
             [1/6/26, 9:20:34 AM] Ann: {L}audio omitted\r\n\
             [1/6/26, 9:20:35 AM] Ann: {L}<attached: 00000005-STICKER-2026-01-06-09-20-35.webp>\r\n\
             [1/6/26, 9:20:36 AM] Ann: hi {L}<This message was edited>\r\n"
        );
        let p = run(&chat);
        assert_eq!(
            kinds(&p),
            [
                Kind::FotoUnica,
                Kind::VideoUnico,
                Kind::AudioUnico,
                Kind::Figurinha,
                Kind::Texto
            ]
        );
        assert_eq!(p.integrity.ignored, [2]);
        assert_eq!(p.events.tbytes[4], 2);
    }

    #[test]
    fn unknown_lrm_body_is_outros() {
        let p = run(&format!(
            "[06/01/2026, 09:20:31] Ana: {L}Ligação de voz perdida\r\n"
        ));
        assert_eq!(kinds(&p), [Kind::Outros]);
        assert_eq!(p.events.tbytes, [0]);
    }

    #[test]
    fn ambiguous_order_uses_chronology() {
        let chat = "[02/01/2026, 10:00:00] A: x\r\n[01/02/2026, 10:00:00] A: x\r\n";
        let p = run(chat);
        assert_eq!(p.integrity.date_order, DateOrder::Dmy);
        assert!(!p.integrity.date_order_guessed);
        let chat = "[01/02/2026, 10:00:00] A: x\r\n[01/03/2026, 10:00:00] A: x\r\n[02/01/2026, 10:00:00] A: x\r\n";
        assert_eq!(run(chat).integrity.date_order, DateOrder::Mdy);
        assert_eq!(
            run("[13/01/2026, 10:00:00] A: x\r\n").integrity.date_order,
            DateOrder::Dmy
        );
        assert_eq!(
            run("[01/13/2026, 10:00:00] A: x\r\n").integrity.date_order,
            DateOrder::Mdy
        );
        let p = run("[01/01/2026, 10:00:00] A: x\r\n");
        assert_eq!(p.integrity.date_order, DateOrder::Dmy);
        assert!(p.integrity.date_order_guessed);
    }

    #[test]
    fn two_digit_year_and_ampm() {
        let p = run("[1/13/26, 12:05:00 AM] A: x\r\n[1/13/26, 1:05:00\u{202F}PM] A: y\r\n[1/13/26, 12:05 p. m.] A: z\r\n");
        let day = days_from_civil(2026, 1, 13) * 86_400;
        assert_eq!(
            p.events.ts,
            [day + 300, day + 12 * 3600 + 300, day + 13 * 3600 + 300]
        );
    }

    #[test]
    fn file_without_crlf() {
        let p = run("[06/01/2026, 09:20:31] Ana: a\nb\n[06/01/2026, 09:20:32] Bia: c\n");
        assert_eq!(p.events.len(), 2);
        assert_eq!(p.events.tbytes, [3, 1]);
    }

    #[test]
    fn crlf_everywhere_counts_inner_breaks_as_one_byte() {
        let p =
            run("[06/01/2026, 09:20:31] Ana: a\r\nb\r\n\r\nc\r\n[06/01/2026, 09:20:32] Bia: d\r\n");
        assert_eq!(p.events.tbytes, [6, 1]);
        assert_eq!(p.events.tchars, [6, 1]);
    }

    #[test]
    fn empty_file() {
        let p = run("");
        assert_eq!(p.events.len(), 0);
        assert_eq!(p.integrity.invalid_lines, 0);
    }

    #[test]
    fn truncated_last_line() {
        let p = run("[06/01/2026, 09:20:31] Ana: a\r\n[06/01/2026, 09:2");
        assert_eq!(p.events.len(), 1);
        assert_eq!(p.events.tbytes, [1]);
        assert_eq!(p.integrity.invalid_lines, 1);
        let p = run("[06/01/2026, 09:20:31] Ana: abc");
        assert_eq!(p.events.tbytes, [3]);
    }

    #[test]
    fn out_of_order_is_sorted_and_refs_follow() {
        let media = MediaIndex::from_entries(vec![Entry {
            name: "a.opus".into(),
            size: 5,
        }]);
        let chat = format!(
            "[06/01/2026, 10:00:00] Ana: {L}<anexado: a.opus>\r\n[06/01/2026, 09:00:00] Ana: x\r\n"
        );
        let p = run_with(chat.as_bytes(), &media);
        assert_eq!(kinds(&p), [Kind::Texto, Kind::Audio]);
        assert_eq!(p.refs, [(1, 0)]);
    }

    #[test]
    fn parallel_split_matches_sequential() {
        let mut chat = String::new();
        for i in 0..5000 {
            chat.push_str(&format!(
                "[{:02}/01/2026, 10:00:00] {}: msg {i} 😂\nmais\r\n",
                i % 28 + 1,
                ["Ana", "Bia"][i % 2]
            ));
        }
        let seq = parse_part(chat.as_bytes(), true, &MediaIndex::default(), true);
        let chunks = split(chat.as_bytes(), true, 7);
        assert_eq!(chunks.len(), 7);
        let par = merge(
            chunks
                .iter()
                .map(|&(s, e)| {
                    parse_part(&chat.as_bytes()[s..e], true, &MediaIndex::default(), true)
                })
                .collect(),
        );
        assert_eq!(seq.ev.tbytes, par.ev.tbytes);
        assert_eq!(seq.emoji_uses, par.emoji_uses);
        assert_eq!(seq.ev.author, par.ev.author);
        assert_eq!(seq.hdr, par.hdr);
    }
}
