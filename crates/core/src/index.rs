use crate::civil::{civil_from_days, days_from_civil, weekday, Day};
use crate::{tz, Analysis, Error, Kind};
use serde::{Deserialize, Serialize};

const K: usize = Kind::ALL.len();
const H: usize = 24;
const A: usize = Kind::Audio as usize;
const MEDIA: [Kind; 7] = [
    Kind::Foto,
    Kind::Video,
    Kind::Figurinha,
    Kind::Gif,
    Kind::Documento,
    Kind::FotoUnica,
    Kind::VideoUnico,
];
const WEEKEND: std::ops::Range<usize> = 5..7;
const NIGHT: std::ops::Range<usize> = 0..5;
const TOP_USED: usize = 3;
const MAX_TIMELINE: usize = 500;
const MAX_PROGRESSION: usize = 1000;

pub struct Index {
    pub persons: Vec<String>,
    rank: Vec<u8>,
    pub origin: String,
    pub display: String,
    first: i64,
    ndays: usize,
    np: usize,
    hourly: Vec<u32>,
    daily: Vec<u32>,
    sums: Vec<[u64; 4]>,
    captioned: Vec<u32>,
    extra: Vec<[u32; 3]>,
    local: Vec<i64>,
    pub dst_ambiguous: u32,
    pub dst_nonexistent: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Query {
    pub from: Option<Day>,
    pub to: Option<Day>,
    pub person: Option<usize>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Totals {
    pub messages: u64,
    pub count: [u64; K],
    pub text_bytes: u64,
    pub text_chars: u64,
    pub media_bytes: [u64; K],
    pub dur_ms: [u64; K],
    pub captioned: u64,
    pub deleted: u64,
    pub links: u64,
    pub emojis: u64,
    pub weekend: u64,
    pub night: u64,
    pub messages_per_day: f64,
    pub audio_min_per_day: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Response {
    pub median_s: Option<f64>,
    pub p75_s: Option<f64>,
    pub p90_s: Option<f64>,
    pub count: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Bucket {
    #[default]
    Dia,
    Semana,
    Mes,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Series {
    pub bucket: Bucket,
    pub labels: Vec<Day>,
    pub total: Vec<u64>,
    pub per_person: Vec<Vec<u64>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct DayValue {
    pub day: Day,
    pub value: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Ranked {
    pub key: u8,
    pub value: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Highlights {
    pub messages: Vec<DayValue>,
    pub audios: Vec<DayValue>,
    pub audio_ms: Vec<DayValue>,
    pub media: Vec<DayValue>,
    pub hours: Vec<Ranked>,
    pub weekdays: Vec<Ranked>,
    pub emojis: Vec<(String, u64)>,
    pub stickers: Vec<(u32, u64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Run {
    pub from: Day,
    pub to: Day,
    pub days: u32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Stats {
    pub from: Option<Day>,
    pub to: Option<Day>,
    pub days: u32,
    pub weekend_days: u32,
    pub persons: Vec<String>,
    pub selected: Vec<bool>,
    pub per_person: Vec<Totals>,
    pub total: Totals,
    pub response: Vec<Response>,
    pub timeline: Series,
    pub progression: Series,
    pub heatmap: Vec<[[u64; H]; 7]>,
    pub heatmap_total: [[u64; H]; 7],
    pub audio_by_hour: Vec<[u64; H]>,
    pub highlights: Highlights,
    pub longest_streak: Option<Run>,
    pub longest_gap: Option<Run>,
    pub extras_pending: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DayRow {
    pub messages: u64,
    pub audios: u64,
    pub audio_ms: u64,
    pub media: u64,
    pub media_bytes: u64,
    pub text_bytes: u64,
}

impl Index {
    pub fn build(a: &Analysis, origin: &str, display: &str) -> Result<Index, Error> {
        let (oz, dz) = (tz::zone(origin)?, tz::zone(display)?);
        let (local, dst_ambiguous, dst_nonexistent) = tz::shift(&oz, &dz, &a.events.ts);
        let deleted_ts: Vec<i64> = a.extras.deleted.iter().map(|d| d.0).collect();
        let deleted = tz::shift(&oz, &dz, &deleted_ts).0;

        let mut order: Vec<usize> = (0..a.authors.len()).collect();
        order.sort_by_cached_key(|&i| a.authors[i].to_lowercase());
        let mut rank = vec![0u8; a.authors.len()];
        for (p, &i) in order.iter().enumerate() {
            rank[i] = p as u8;
        }
        let np = a.authors.len();
        let day_of = |t: i64| t.div_euclid(86_400);
        let all = || local.iter().chain(&deleted).copied();
        let first = all().min().map_or(0, day_of);
        let ndays = all().max().map_or(0, |t| (day_of(t) - first + 1) as usize);

        let mut ix = Index {
            persons: order.iter().map(|&i| a.authors[i].clone()).collect(),
            rank,
            origin: origin.into(),
            display: display.into(),
            first,
            ndays,
            np,
            hourly: vec![0; ndays * H * np * K],
            daily: vec![0; ndays * np * K],
            sums: vec![[0; 4]; ndays * np * K],
            captioned: vec![0; ndays * np],
            extra: vec![[0; 3]; ndays * np],
            local,
            dst_ambiguous,
            dst_nonexistent,
        };
        let e = &a.events;
        for (i, &t) in ix.local.iter().enumerate() {
            let d = (day_of(t) - first) as usize;
            let h = (t.rem_euclid(86_400) / 3600) as usize;
            let p = usize::from(ix.rank[usize::from(e.author[i])]);
            let k = usize::from(e.kind[i]).min(K - 1);
            ix.hourly[((d * H + h) * np + p) * K + k] += 1;
            ix.daily[(d * np + p) * K + k] += 1;
            let s = &mut ix.sums[(d * np + p) * K + k];
            s[0] += u64::from(e.tbytes[i]);
            s[1] += u64::from(e.tchars[i]);
            s[2] += e.msize[i];
            s[3] += u64::from(e.dur_ms[i]);
        }
        for &c in &a.captioned {
            let c = c as usize;
            let d = (day_of(ix.local[c]) - first) as usize;
            ix.captioned[d * np + usize::from(ix.rank[usize::from(e.author[c])])] += 1;
        }
        let x = &a.extras;
        let cell = |ix: &Index, t: i64, author: u8| {
            (day_of(t) - first) as usize * np + usize::from(ix.rank[usize::from(author)])
        };
        for (&t, &(_, author)) in deleted.iter().zip(&x.deleted) {
            let c = cell(&ix, t, author);
            ix.extra[c][0] += 1;
        }
        for &(ev, n) in &x.links {
            let c = cell(&ix, ix.local[ev as usize], e.author[ev as usize]);
            ix.extra[c][1] += n;
        }
        for &(ev, _, n) in &x.emoji_uses {
            let c = cell(&ix, ix.local[ev as usize], e.author[ev as usize]);
            ix.extra[c][2] += n;
        }
        Ok(ix)
    }

    pub fn person_of(&self, author: usize) -> usize {
        usize::from(self.rank[author])
    }

    pub fn span(&self) -> Option<(Day, Day)> {
        (self.ndays > 0).then(|| (Day(self.first), Day(self.first + self.ndays as i64 - 1)))
    }

    fn resolve(&self, q: &Query) -> (Option<(usize, usize)>, Vec<bool>) {
        let mask = (0..self.np)
            .map(|p| q.person.is_none_or(|s| s == p))
            .collect();
        if self.ndays == 0 {
            return (None, mask);
        }
        let last = self.first + self.ndays as i64 - 1;
        let from = q.from.map_or(self.first, |d| d.0).max(self.first);
        let to = q.to.map_or(last, |d| d.0).min(last);
        let rows = (from <= to).then(|| ((from - self.first) as usize, (to - self.first) as usize));
        (rows, mask)
    }

    pub fn stats(&self, a: &Analysis, q: &Query) -> Stats {
        let np = self.np;
        let (rows, selected) = self.resolve(q);
        let mut s = Stats {
            from: q.from,
            to: q.to,
            persons: self.persons.clone(),
            per_person: vec![Totals::default(); np],
            response: vec![Response::default(); np],
            heatmap: vec![[[0; H]; 7]; np],
            audio_by_hour: vec![[0; H]; np],
            selected,
            extras_pending: a.extras_pending,
            ..Default::default()
        };
        let Some((lo, hi)) = rows else {
            return s;
        };
        let n = hi - lo + 1;
        s.weekend_days = (lo..=hi)
            .filter(|&d| WEEKEND.contains(&weekday(self.first + d as i64)))
            .count() as u32;
        s.from = Some(Day(self.first + lo as i64));
        s.to = Some(Day(self.first + hi as i64));
        s.days = n as u32;

        let mut msgs = vec![0u64; n];
        let (mut audios, mut audio_ms, mut media) = (vec![0u64; n], vec![0u64; n], vec![0u64; n]);
        let mut per_day = vec![vec![0u64; n]; np];

        for d in lo..=hi {
            let i = d - lo;
            let wd = weekday(self.first + d as i64);
            for (p, pd) in per_day.iter_mut().enumerate() {
                let at = (d * np + p) * K;
                let row = &self.daily[at..at + K];
                let sums = &self.sums[at..at + K];
                let t = &mut s.per_person[p];
                for k in 0..K {
                    t.count[k] += u64::from(row[k]);
                    t.text_bytes += sums[k][0];
                    t.text_chars += sums[k][1];
                    t.media_bytes[k] += sums[k][2];
                    t.dur_ms[k] += sums[k][3];
                }
                t.captioned += u64::from(self.captioned[d * np + p]);
                let [del, links, emojis] = self.extra[d * np + p];
                t.deleted += u64::from(del);
                t.links += u64::from(links);
                t.emojis += u64::from(emojis);
                let m: u64 = row.iter().map(|&c| u64::from(c)).sum();
                pd[i] = m;
                if s.selected[p] {
                    msgs[i] += m;
                    audios[i] += u64::from(row[A]);
                    audio_ms[i] += sums[A][3];
                    media[i] += MEDIA
                        .iter()
                        .map(|&k| u64::from(row[k as usize]))
                        .sum::<u64>();
                }
                for h in 0..H {
                    let at = ((d * H + h) * np + p) * K;
                    let cell = &self.hourly[at..at + K];
                    let c: u64 = cell.iter().map(|&c| u64::from(c)).sum();
                    s.heatmap[p][wd][h] += c;
                    s.audio_by_hour[p][h] += u64::from(cell[A]);
                }
            }
        }

        for (p, t) in s.per_person.iter_mut().enumerate() {
            let hm = &s.heatmap[p];
            t.weekend = hm[WEEKEND].iter().flatten().sum();
            t.night = hm.iter().flat_map(|w| &w[NIGHT]).sum();
            finish(t, n);
            if s.selected[p] {
                add(&mut s.total, t);
                for wd in 0..7 {
                    for h in 0..H {
                        s.heatmap_total[wd][h] += s.heatmap[p][wd][h];
                    }
                }
            }
        }
        finish(&mut s.total, n);

        s.response = self.response(a, lo, hi);
        s.timeline = self.timeline(lo, &msgs, &per_day);
        s.progression = progression(self.first + lo as i64, &msgs, &per_day);
        s.highlights = Highlights {
            messages: top10(&msgs, self.first + lo as i64),
            audios: top10(&audios, self.first + lo as i64),
            audio_ms: top10(&audio_ms, self.first + lo as i64),
            media: top10(&media, self.first + lo as i64),
            hours: ranked((0..H).map(|h| (0..7).map(|w| s.heatmap_total[w][h]).sum())),
            weekdays: ranked(s.heatmap_total.iter().map(|w| w.iter().sum())),
            ..Default::default()
        };
        self.top_used(a, lo, hi, &mut s);
        (s.longest_streak, s.longest_gap) = runs(&msgs, self.first + lo as i64);
        s
    }

    fn events_in(&self, lo: usize, hi: usize) -> std::ops::Range<usize> {
        let start = (self.first + lo as i64) * 86_400;
        let end = (self.first + hi as i64 + 1) * 86_400;
        self.local.partition_point(|&t| t < start)..self.local.partition_point(|&t| t < end)
    }

    fn top_used(&self, a: &Analysis, lo: usize, hi: usize, s: &mut Stats) {
        let r = self.events_in(lo, hi);
        let x = &a.extras;
        let selected = &s.selected;
        let sel = |ev: u32| selected[self.person_of(usize::from(a.events.author[ev as usize]))];
        let within = |ev: u32| r.contains(&(ev as usize));

        let mut uses = vec![0u64; x.emojis.len()];
        let at = x.emoji_uses.partition_point(|u| (u.0 as usize) < r.start);
        for &(ev, id, n) in x.emoji_uses[at..].iter().take_while(|u| within(u.0)) {
            if sel(ev) {
                uses[id as usize] += u64::from(n);
            }
        }
        let emojis = top(&uses)
            .into_iter()
            .map(|(id, n)| (x.emojis[id].clone(), n))
            .collect();

        let mut uses = vec![0u64; x.sticker_files.len()];
        let at = x.stickers.partition_point(|u| (u.0 as usize) < r.start);
        for &(ev, id) in x.stickers[at..].iter().take_while(|u| within(u.0)) {
            if sel(ev) {
                uses[id as usize] += 1;
            }
        }
        s.highlights.stickers = top(&uses)
            .into_iter()
            .map(|(id, n)| (id as u32, n))
            .collect();
        s.highlights.emojis = emojis;
    }

    fn response(&self, a: &Analysis, lo: usize, hi: usize) -> Vec<Response> {
        let std::ops::Range { start: i0, end: i1 } = self.events_in(lo, hi);
        let e = &a.events;
        let mut gaps: Vec<Vec<i64>> = vec![Vec::new(); self.np];
        for i in i0 + 1..i1 {
            let (prev, cur) = (e.author[i - 1], e.author[i]);
            if prev != cur {
                let p = usize::from(self.rank[usize::from(cur)]);
                gaps[p].push((e.ts[i] - e.ts[i - 1]).max(0));
            }
        }
        gaps.into_iter()
            .map(|mut g| Response {
                count: g.len() as u64,
                p75_s: percentile(&mut g, 75),
                p90_s: percentile(&mut g, 90),
                median_s: median(&mut g),
            })
            .collect()
    }

    fn timeline(&self, lo: usize, msgs: &[u64], per_day: &[Vec<u64>]) -> Series {
        let n = msgs.len();
        let bucket = if n <= MAX_TIMELINE {
            Bucket::Dia
        } else if n.div_ceil(7) <= MAX_TIMELINE {
            Bucket::Semana
        } else {
            Bucket::Mes
        };
        let start_of = |day: i64| match bucket {
            Bucket::Dia => day,
            Bucket::Semana => day - weekday(day) as i64,
            Bucket::Mes => {
                let (y, m, _) = civil_from_days(day);
                days_from_civil(y, i64::from(m), 1)
            }
        };
        let mut s = Series {
            bucket,
            per_person: vec![Vec::new(); per_day.len()],
            ..Default::default()
        };
        for (i, &m) in msgs.iter().enumerate() {
            let b = start_of(self.first + (lo + i) as i64);
            if s.labels.last() != Some(&Day(b)) {
                s.labels.push(Day(b));
                s.total.push(0);
                s.per_person.iter_mut().for_each(|v| v.push(0));
            }
            *s.total.last_mut().unwrap() += m;
            for (p, v) in s.per_person.iter_mut().enumerate() {
                *v.last_mut().unwrap() += per_day[p][i];
            }
        }
        s
    }

    pub fn daily(&self, q: &Query) -> Vec<(Day, Vec<DayRow>)> {
        let Some((lo, hi)) = self.resolve(q).0 else {
            return Vec::new();
        };
        (lo..=hi)
            .map(|d| {
                let rows = (0..self.np)
                    .map(|p| {
                        let at = (d * self.np + p) * K;
                        let (c, s) = (&self.daily[at..at + K], &self.sums[at..at + K]);
                        DayRow {
                            messages: c.iter().map(|&x| u64::from(x)).sum(),
                            audios: u64::from(c[A]),
                            audio_ms: s[A][3],
                            media: MEDIA.iter().map(|&k| u64::from(c[k as usize])).sum(),
                            media_bytes: s.iter().map(|x| x[2]).sum(),
                            text_bytes: s.iter().map(|x| x[0]).sum(),
                        }
                    })
                    .collect();
                (Day(self.first + d as i64), rows)
            })
            .collect()
    }
}

fn add(t: &mut Totals, o: &Totals) {
    for k in 0..K {
        t.count[k] += o.count[k];
        t.media_bytes[k] += o.media_bytes[k];
        t.dur_ms[k] += o.dur_ms[k];
    }
    t.text_bytes += o.text_bytes;
    t.text_chars += o.text_chars;
    t.captioned += o.captioned;
    t.deleted += o.deleted;
    t.links += o.links;
    t.emojis += o.emojis;
    t.weekend += o.weekend;
    t.night += o.night;
}

fn top(uses: &[u64]) -> Vec<(usize, u64)> {
    let mut v: Vec<(usize, u64)> = uses
        .iter()
        .copied()
        .enumerate()
        .filter(|&(_, n)| n > 0)
        .collect();
    let cmp = |a: &(usize, u64), b: &(usize, u64)| b.1.cmp(&a.1).then(a.0.cmp(&b.0));
    if v.len() > TOP_USED {
        v.select_nth_unstable_by(TOP_USED - 1, cmp);
        v.truncate(TOP_USED);
    }
    v.sort_unstable_by(cmp);
    v
}

fn finish(t: &mut Totals, days: usize) {
    t.messages = t.count.iter().sum();
    t.messages_per_day = t.messages as f64 / days as f64;
    t.audio_min_per_day = t.dur_ms[A] as f64 / 60_000.0 / days as f64;
}

fn median(v: &mut [i64]) -> Option<f64> {
    let n = v.len();
    if n == 0 {
        return None;
    }
    let (left, &mut hi, _) = v.select_nth_unstable(n / 2);
    Some(if n % 2 == 1 {
        hi as f64
    } else {
        (left.iter().copied().max().unwrap_or(hi) + hi) as f64 / 2.0
    })
}

fn percentile(v: &mut [i64], pct: usize) -> Option<f64> {
    let n = v.len();
    if n == 0 {
        return None;
    }
    let rank = (n * pct).div_ceil(100).max(1);
    Some(*v.select_nth_unstable(rank - 1).1 as f64)
}

fn progression(first: i64, msgs: &[u64], per_day: &[Vec<u64>]) -> Series {
    let n = msgs.len();
    let step = n.div_ceil(MAX_PROGRESSION - 1).max(1);
    let keep = |i: usize| i.is_multiple_of(step) || i + 1 == n;
    let cumulative = |v: &[u64]| -> Vec<u64> {
        let mut acc = 0;
        v.iter()
            .enumerate()
            .filter_map(|(i, &x)| {
                acc += x;
                keep(i).then_some(acc)
            })
            .collect()
    };
    Series {
        bucket: Bucket::Dia,
        labels: (0..n)
            .filter(|&i| keep(i))
            .map(|i| Day(first + i as i64))
            .collect(),
        total: cumulative(msgs),
        per_person: per_day.iter().map(|v| cumulative(v)).collect(),
    }
}

fn top10(v: &[u64], first: i64) -> Vec<DayValue> {
    let mut d: Vec<DayValue> = v
        .iter()
        .enumerate()
        .filter(|(_, &x)| x > 0)
        .map(|(i, &value)| DayValue {
            day: Day(first + i as i64),
            value,
        })
        .collect();
    let cmp = |a: &DayValue, b: &DayValue| b.value.cmp(&a.value).then(a.day.cmp(&b.day));
    if d.len() > 10 {
        d.select_nth_unstable_by(9, cmp);
        d.truncate(10);
    }
    d.sort_unstable_by(cmp);
    d
}

fn ranked(v: impl Iterator<Item = u64>) -> Vec<Ranked> {
    let mut r: Vec<Ranked> = v
        .enumerate()
        .map(|(k, value)| Ranked {
            key: k as u8,
            value,
        })
        .collect();
    r.sort_by(|a, b| b.value.cmp(&a.value).then(a.key.cmp(&b.key)));
    r
}

fn runs(msgs: &[u64], first: i64) -> (Option<Run>, Option<Run>) {
    let (mut streak, mut gap): (Option<Run>, Option<Run>) = (None, None);
    let mut i = 0;
    while i < msgs.len() {
        let active = msgs[i] > 0;
        let j = (i..msgs.len())
            .find(|&j| (msgs[j] > 0) != active)
            .unwrap_or(msgs.len());
        let run = Run {
            from: Day(first + i as i64),
            to: Day(first + j as i64 - 1),
            days: (j - i) as u32,
        };
        let best = if active {
            &mut streak
        } else if i > 0 && j < msgs.len() {
            &mut gap
        } else {
            i = j;
            continue;
        };
        if best.is_none_or(|b| run.days > b.days) {
            *best = Some(run);
        }
        i = j;
    }
    (streak, gap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Events, Extras, Integrity};

    fn at(y: i64, m: i64, d: i64, h: i64) -> i64 {
        days_from_civil(y, m, d) * 86_400 + h * 3600
    }

    fn analysis(ev: &[(i64, u8, Kind, u32)]) -> Analysis {
        let mut e = Events::default();
        for &(t, a, k, d) in ev {
            e.ts.push(t);
            e.author.push(a);
            e.kind.push(k as u8);
            e.tbytes.push(if k == Kind::Texto { 3 } else { 0 });
            e.tchars.push(if k == Kind::Texto { 3 } else { 0 });
            e.msize.push(if k == Kind::Texto { 0 } else { 100 });
            e.dur_ms.push(d);
        }
        Analysis {
            authors: vec!["Bia".into(), "Ana".into()],
            events: e,
            captioned: vec![],
            extras: Extras::default(),
            integrity: Integrity::default(),
            chat_sha256: [0; 32],
            analyzed_at: 0,
            timings: vec![],
            from_cache: false,
            extras_pending: false,
        }
    }

    fn sample() -> Analysis {
        analysis(&[
            (at(2026, 1, 5, 9), 1, Kind::Texto, 0),
            (at(2026, 1, 5, 10), 0, Kind::Texto, 0),
            (at(2026, 1, 5, 10), 0, Kind::Audio, 60_000),
            (at(2026, 1, 6, 9), 1, Kind::Foto, 0),
            (at(2026, 1, 9, 23), 0, Kind::Audio, 30_000),
        ])
    }

    #[test]
    fn totals_persons_and_filter() {
        let a = sample();
        let ix = Index::build(&a, "UTC", "UTC").unwrap();
        assert_eq!(ix.persons, ["Ana", "Bia"]);
        let s = ix.stats(&a, &Query::default());
        assert_eq!(s.days, 5);
        assert_eq!(s.total.messages, 5);
        assert_eq!(s.per_person[0].count[Kind::Foto as usize], 1);
        assert_eq!(s.per_person[1].dur_ms[A], 90_000);
        assert_eq!(s.total.text_bytes, 6);
        assert!((s.total.messages_per_day - 1.0).abs() < 1e-9);
        assert_eq!(s.heatmap[1][0][10], 2);
        assert_eq!(s.audio_by_hour[1][23], 1);

        let only_ana = ix.stats(
            &a,
            &Query {
                person: Some(0),
                ..Default::default()
            },
        );
        assert_eq!(only_ana.total.messages, 2);
        assert_eq!(only_ana.per_person[1].count[A], 2);
        assert_eq!(only_ana.highlights.audios, []);
    }

    #[test]
    fn period_and_zone() {
        let a = sample();
        let ix = Index::build(&a, "UTC", "UTC").unwrap();
        let q = Query {
            from: Day::parse("2026-01-06"),
            to: Day::parse("2026-01-06"),
            person: None,
        };
        let s = ix.stats(&a, &q);
        assert_eq!((s.days, s.total.messages), (1, 1));
        let ix = Index::build(&a, "UTC", "Asia/Tokyo").unwrap();
        assert_eq!(ix.span().unwrap().1, Day::parse("2026-01-10").unwrap());
        let q = Query {
            from: Day::parse("2027-01-01"),
            ..Default::default()
        };
        let s = ix.stats(&a, &q);
        assert_eq!((s.days, s.total.messages), (0, 0));
    }

    #[test]
    fn response_median() {
        let a = sample();
        let ix = Index::build(&a, "UTC", "UTC").unwrap();
        let s = ix.stats(&a, &Query::default());
        assert_eq!(s.response[0].count, 1);
        assert_eq!(s.response[0].median_s, Some(23.0 * 3600.0));
        assert_eq!(s.response[1].count, 2);
        assert_eq!(s.response[1].median_s, Some((3600.0 + 86.0 * 3600.0) / 2.0));
        assert_eq!(median(&mut [5, 1, 3]), Some(3.0));
        assert_eq!(median(&mut []), None);
        let mut v: Vec<i64> = (1..=10).rev().collect();
        assert_eq!(percentile(&mut v, 75), Some(8.0));
        assert_eq!(percentile(&mut v, 90), Some(9.0));
        assert_eq!(percentile(&mut [42], 90), Some(42.0));
        assert_eq!(percentile(&mut [], 75), None);
    }

    #[test]
    fn highlights_and_runs() {
        let a = sample();
        let ix = Index::build(&a, "UTC", "UTC").unwrap();
        let s = ix.stats(&a, &Query::default());
        let d = |x: &str| Day::parse(x).unwrap();
        assert_eq!(
            s.highlights.messages[0],
            DayValue {
                day: d("2026-01-05"),
                value: 3
            }
        );
        assert_eq!(s.highlights.audio_ms[0].value, 60_000);
        assert_eq!(
            s.highlights.media,
            [DayValue {
                day: d("2026-01-06"),
                value: 1
            }]
        );
        assert_eq!(s.highlights.weekdays[0], Ranked { key: 0, value: 3 });
        assert_eq!(s.highlights.hours[0].key, 9);
        assert_eq!(
            s.longest_streak,
            Some(Run {
                from: d("2026-01-05"),
                to: d("2026-01-06"),
                days: 2
            })
        );
        assert_eq!(
            s.longest_gap,
            Some(Run {
                from: d("2026-01-07"),
                to: d("2026-01-08"),
                days: 2
            })
        );
    }

    #[test]
    fn extras_weekend_and_night() {
        let mut a = sample();
        a.extras.deleted = vec![(at(2026, 1, 10, 3), 1)];
        a.extras.links = vec![(0, 2)];
        a.extras.emojis = vec!["😂".into(), "❤️".into()];
        a.extras.emoji_uses = vec![(0, 1, 1), (0, 0, 3), (1, 1, 2)];
        a.extras.sticker_files = vec!["a.webp".into(), "b.webp".into()];
        a.extras.stickers = vec![(1, 1), (2, 1), (3, 0)];
        let ix = Index::build(&a, "UTC", "UTC").unwrap();
        let s = ix.stats(&a, &Query::default());
        assert_eq!(s.days, 6);
        assert_eq!(s.weekend_days, 1);
        let (ana, bia) = (&s.per_person[0], &s.per_person[1]);
        assert_eq!((ana.deleted, ana.links, ana.emojis), (1, 2, 4));
        assert_eq!((bia.deleted, bia.links, bia.emojis), (0, 0, 2));
        assert_eq!(s.total.emojis, 6);
        assert_eq!((ana.weekend, ana.night, bia.night), (0, 0, 0));
        assert_eq!(
            s.highlights.emojis,
            [("😂".to_string(), 3), ("❤️".to_string(), 3)]
        );
        assert_eq!(s.highlights.stickers, [(1, 2), (0, 1)]);
        let only_bia = ix.stats(
            &a,
            &Query {
                person: Some(1),
                ..Default::default()
            },
        );
        assert_eq!(only_bia.highlights.emojis, [("❤️".to_string(), 2)]);
        assert_eq!(only_bia.highlights.stickers, [(1, 2)]);
        let q = Query {
            from: Day::parse("2026-01-06"),
            ..Default::default()
        };
        let later = ix.stats(&a, &q);
        assert_eq!(later.highlights.emojis, []);
        assert_eq!(later.highlights.stickers, [(0, 1)]);

        let night = analysis(&[(at(2026, 1, 10, 4), 0, Kind::Texto, 0)]);
        let s = Index::build(&night, "UTC", "UTC")
            .unwrap()
            .stats(&night, &Query::default());
        assert_eq!((s.total.weekend, s.total.night, s.weekend_days), (1, 1, 1));
        assert_eq!(top(&[0, 5, 1, 5, 2]), [(1, 5), (3, 5), (4, 2)]);
    }

    #[test]
    fn series_resampling() {
        let ev: Vec<_> = (0..4000)
            .map(|i| {
                (
                    at(2020, 1, 1, 12) + i * 86_400,
                    (i % 2) as u8,
                    Kind::Texto,
                    0,
                )
            })
            .collect();
        let a = analysis(&ev);
        let ix = Index::build(&a, "UTC", "UTC").unwrap();
        let s = ix.stats(&a, &Query::default());
        assert_eq!(s.timeline.bucket, Bucket::Mes);
        assert!(s.timeline.labels.len() <= MAX_TIMELINE);
        assert_eq!(s.timeline.total.iter().sum::<u64>(), 4000);
        assert!(s.progression.labels.len() <= MAX_PROGRESSION);
        assert_eq!(*s.progression.total.last().unwrap(), 4000);
        assert_eq!(*s.progression.per_person[0].last().unwrap(), 2000);
        let top: Vec<u64> = (0..30).collect();
        let t = top10(&top, 0);
        assert_eq!(t.len(), 10);
        assert_eq!(t[0].value, 29);
        assert_eq!(t[9].value, 20);
    }
}
