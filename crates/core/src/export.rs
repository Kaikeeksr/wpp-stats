use crate::civil::Day;
use crate::index::{DayRow, DayValue, Index, Query, Response, Run, Stats, Totals};
use crate::{hex, io, Analysis, DateOrder, Error, Kind};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const KIND_LABELS: [&str; Kind::ALL.len()] = [
    "Texto",
    "Áudio",
    "Foto",
    "Vídeo",
    "Figurinha",
    "GIF",
    "Documento",
    "Foto de visualização única",
    "Vídeo de visualização única",
    "Áudio de visualização única",
    "Outros",
];
pub const WEEKDAYS: [&str; 7] = [
    "Segunda", "Terça", "Quarta", "Quinta", "Sexta", "Sábado", "Domingo",
];

pub fn fmt_int(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

pub fn fmt_dec(x: f64, places: usize) -> String {
    format!("{x:.places$}").replace('.', ",")
}

pub fn fmt_dur(ms: u64) -> String {
    let s = ms / 1000;
    match (s / 3600, s % 3600 / 60) {
        (0, 0) => format!("{s}s"),
        (0, m) => format!("{m}min"),
        (h, m) => format!("{}h {m:02}min", fmt_int(h)),
    }
}

pub fn fmt_secs(s: f64) -> String {
    let s = s.round() as u64;
    match (s / 3600, s % 3600 / 60, s % 60) {
        (0, 0, x) => format!("{x}s"),
        (0, m, x) => format!("{m}min {x:02}s"),
        (h, m, _) => format!("{}h {m:02}min", fmt_int(h)),
    }
}

pub fn fmt_size(b: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut x = b as f64;
    let mut u = 0;
    while x >= 1024.0 && u < UNITS.len() - 1 {
        x /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{b} B")
    } else {
        format!("{} {}", fmt_dec(x, 1), UNITS[u])
    }
}

#[derive(Serialize)]
pub struct IntegrityReport<'a> {
    pub missing: &'a [String],
    pub orphans: &'a [String],
    pub no_duration: &'a [String],
    pub outros: Vec<u64>,
    pub captioned: u64,
    pub ignored: Vec<u32>,
    pub dst_ambiguous: u32,
    pub dst_nonexistent: u32,
    pub date_order: &'static str,
    pub date_order_guessed: bool,
    pub invalid_lines: u32,
    pub no_author: u32,
    pub bad_dates: u32,
}

#[derive(Serialize)]
struct DailyJson<'a> {
    day: Day,
    per_person: &'a [DayRow],
}

#[derive(Serialize)]
struct JsonReport<'a> {
    schema_version: u32,
    app_version: &'a str,
    folder: String,
    analyzed_at: String,
    origin_tz: &'a str,
    display_tz: &'a str,
    query: &'a Query,
    kinds: [&'a str; Kind::ALL.len()],
    stats: &'a Stats,
    daily: Vec<DailyJson<'a>>,
    integrity: IntegrityReport<'a>,
    chat_sha256: String,
}

pub struct Report<'a> {
    pub a: &'a Analysis,
    pub ix: &'a Index,
    pub q: Query,
    pub folder: &'a Path,
    pub stats: Stats,
    pub anonymous: bool,
    daily: Vec<(Day, Vec<DayRow>)>,
}

impl<'a> Report<'a> {
    pub fn new(a: &'a Analysis, ix: &'a Index, q: Query, folder: &'a Path) -> Self {
        let stats = ix.stats(a, &q);
        let daily = ix.daily(&q);
        Report {
            a,
            ix,
            q,
            folder,
            stats,
            anonymous: false,
            daily,
        }
    }

    fn sel(&self) -> Vec<usize> {
        (0..self.stats.persons.len())
            .filter(|&p| self.stats.selected[p])
            .collect()
    }

    pub fn person(&self, i: usize) -> String {
        match (self.anonymous, u8::try_from(i)) {
            (false, _) => self.stats.persons[i].clone(),
            (true, Ok(n)) if n < 26 => format!("Pessoa {}", char::from(b'A' + n)),
            (true, _) => format!("Pessoa {}", i + 1),
        }
    }

    fn folder_name(&self) -> String {
        if self.anonymous {
            return "Conversa".into();
        }
        self.folder.file_name().map_or_else(
            || self.folder.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    }

    fn analyzed_at(&self) -> String {
        let tz = crate::tz::zone(&self.ix.display).unwrap_or(jiff::tz::TimeZone::UTC);
        jiff::Timestamp::from_second(self.a.analyzed_at)
            .map(|t| t.to_zoned(tz).strftime("%d/%m/%Y %H:%M").to_string())
            .unwrap_or_default()
    }

    fn period(&self) -> String {
        match (self.stats.from, self.stats.to) {
            (Some(f), Some(t)) => format!("{} a {}", f.br(), t.br()),
            _ => "sem mensagens".into(),
        }
    }

    pub fn integrity(&self) -> IntegrityReport<'a> {
        let i = &self.a.integrity;
        let np = self.stats.persons.len();
        let (mut outros, mut ignored) = (vec![0; np], vec![0; np]);
        let e = &self.a.events;
        for (j, &k) in e.kind.iter().enumerate() {
            if k == Kind::Outros as u8 {
                outros[self.ix.person_of(usize::from(e.author[j]))] += 1;
            }
        }
        for (author, &n) in i.ignored.iter().enumerate() {
            ignored[self.ix.person_of(author)] += n;
        }
        IntegrityReport {
            missing: &i.missing,
            orphans: &i.orphans,
            no_duration: &i.no_duration,
            outros,
            captioned: self.a.captioned.len() as u64,
            ignored,
            dst_ambiguous: self.ix.dst_ambiguous,
            dst_nonexistent: self.ix.dst_nonexistent,
            date_order: match i.date_order {
                DateOrder::Dmy => "dd/mm",
                DateOrder::Mdy => "mm/dd",
            },
            date_order_guessed: i.date_order_guessed,
            invalid_lines: i.invalid_lines,
            no_author: i.no_author,
            bad_dates: i.bad_dates,
        }
    }

    pub fn json(&self) -> String {
        let r = JsonReport {
            schema_version: SCHEMA_VERSION,
            app_version: APP_VERSION,
            folder: self.folder.display().to_string(),
            analyzed_at: self.analyzed_at(),
            origin_tz: &self.ix.origin,
            display_tz: &self.ix.display,
            query: &self.q,
            kinds: KIND_LABELS,
            stats: &self.stats,
            daily: self
                .daily
                .iter()
                .map(|(day, rows)| DailyJson {
                    day: *day,
                    per_person: rows,
                })
                .collect(),
            integrity: self.integrity(),
            chat_sha256: hex(&self.a.chat_sha256),
        };
        serde_json::to_string_pretty(&r).expect("plain data serializes")
    }

    pub fn write_json(&self, path: &Path) -> Result<(), Error> {
        std::fs::write(path, self.json()).map_err(io(path))
    }

    pub fn write_csv(&self, dir: &Path) -> Result<Vec<PathBuf>, Error> {
        std::fs::create_dir_all(dir).map_err(io(dir))?;
        let tables = [
            ("resumo_por_pessoa.csv", self.csv_summary()),
            ("diario.csv", self.csv_daily()),
            ("hora_dia_semana.csv", self.csv_heatmap()),
            ("top10.csv", self.csv_top10()),
        ];
        let mut out = Vec::new();
        for (name, rows) in tables {
            let path = dir.join(name);
            write_csv_file(&path, &rows)?;
            out.push(path);
        }
        Ok(out)
    }

    fn csv_summary(&self) -> Vec<Vec<String>> {
        let s = &self.stats;
        let sel = self.sel();
        let mut head = vec!["Métrica".to_string()];
        head.extend(sel.iter().map(|&p| s.persons[p].clone()));
        head.push("Total".into());
        let mut rows = vec![head];
        let mut row = |label: &str, f: &dyn Fn(&Totals) -> String| {
            let mut r = vec![label.to_string()];
            r.extend(sel.iter().map(|&p| f(&s.per_person[p])));
            r.push(f(&s.total));
            rows.push(r);
        };
        let min = |ms: u64| fmt_dec(ms as f64 / 60_000.0, 1);
        row("Mensagens", &|t| t.messages.to_string());
        for (k, label) in KIND_LABELS.iter().enumerate() {
            row(label, &|t| t.count[k].to_string());
        }
        row("Bytes de texto", &|t| t.text_bytes.to_string());
        row("Caracteres de texto", &|t| t.text_chars.to_string());
        row("Minutos de áudio", &|t| min(t.dur_ms[Kind::Audio as usize]));
        row("Minutos de vídeo", &|t| min(t.dur_ms[Kind::Video as usize]));
        for (k, label) in KIND_LABELS.iter().enumerate().skip(1).take(6) {
            row(&format!("Bytes de {}", label.to_lowercase()), &|t| {
                t.media_bytes[k].to_string()
            });
        }
        row("Bytes de mídia (total)", &|t| {
            t.media_bytes.iter().sum::<u64>().to_string()
        });
        row("Mídias com legenda", &|t| t.captioned.to_string());
        row("Mensagens apagadas", &|t| t.deleted.to_string());
        row("Links", &|t| t.links.to_string());
        row("Emojis", &|t| t.emojis.to_string());
        row("Mensagens no fim de semana", &|t| t.weekend.to_string());
        row("Mensagens na madrugada (0h a 4h59)", &|t| {
            t.night.to_string()
        });
        row("Mensagens por dia", &|t| fmt_dec(t.messages_per_day, 2));
        row("Minutos de áudio por dia", &|t| {
            fmt_dec(t.audio_min_per_day, 2)
        });
        let secs = |x: Option<f64>| x.map_or_else(String::new, |x| fmt_dec(x, 1));
        let rsp = |label: &str, f: &dyn Fn(&Response) -> String| {
            let mut r = vec![label.to_string()];
            r.extend(sel.iter().map(|&p| f(&s.response[p])));
            r.push(String::new());
            r
        };
        rows.extend([
            rsp("Resposta em até (s), 50% das vezes (mediana)", &|r| {
                secs(r.median_s)
            }),
            rsp("Resposta em até (s), 75% das vezes", &|r| secs(r.p75_s)),
            rsp("Resposta em até (s), 90% das vezes", &|r| secs(r.p90_s)),
            rsp("Respostas consideradas", &|r| r.count.to_string()),
        ]);
        rows
    }

    fn csv_daily(&self) -> Vec<Vec<String>> {
        let s = &self.stats;
        let sel = self.sel();
        let mut head = vec!["Data".to_string()];
        for &p in &sel {
            let n = &s.persons[p];
            for col in [
                "Mensagens",
                "Áudios",
                "Minutos de áudio",
                "Mídias",
                "Bytes de mídia",
                "Bytes de texto",
            ] {
                head.push(format!("{col} - {n}"));
            }
        }
        head.push("Mensagens - Total".into());
        let mut rows = vec![head];
        for (day, per) in &self.daily {
            let mut r = vec![day.br()];
            let mut total = 0;
            for &p in &sel {
                let d = &per[p];
                total += d.messages;
                r.extend([
                    d.messages.to_string(),
                    d.audios.to_string(),
                    fmt_dec(d.audio_ms as f64 / 60_000.0, 1),
                    d.media.to_string(),
                    d.media_bytes.to_string(),
                    d.text_bytes.to_string(),
                ]);
            }
            r.push(total.to_string());
            rows.push(r);
        }
        rows
    }

    fn csv_heatmap(&self) -> Vec<Vec<String>> {
        let mut head = vec!["Dia da semana".to_string()];
        head.extend((0..24).map(|h| format!("{h}h")));
        let mut rows = vec![head];
        for (w, name) in WEEKDAYS.iter().enumerate() {
            let mut r = vec![name.to_string()];
            r.extend(self.stats.heatmap_total[w].iter().map(u64::to_string));
            rows.push(r);
        }
        rows
    }

    fn csv_top10(&self) -> Vec<Vec<String>> {
        let h = &self.stats.highlights;
        let mut rows = vec![vec![
            "Categoria".into(),
            "Posição".into(),
            "Data".into(),
            "Valor".into(),
        ]];
        let lists: [(&str, &Vec<DayValue>, bool); 4] = [
            ("Mensagens", &h.messages, false),
            ("Áudios", &h.audios, false),
            ("Minutos de áudio", &h.audio_ms, true),
            ("Mídias", &h.media, false),
        ];
        for (cat, list, minutes) in lists {
            for (i, dv) in list.iter().enumerate() {
                let v = if minutes {
                    fmt_dec(dv.value as f64 / 60_000.0, 1)
                } else {
                    dv.value.to_string()
                };
                rows.push(vec![cat.into(), (i + 1).to_string(), dv.day.br(), v]);
            }
        }
        rows
    }

    pub fn write_pdf(&self, path: &Path) -> Result<(), Error> {
        std::fs::write(path, self.pdf()?).map_err(io(path))
    }

    pub fn pdf(&self) -> Result<Vec<u8>, Error> {
        pdf::render(self)
    }
}

fn write_csv_file(path: &Path, rows: &[Vec<String>]) -> Result<(), Error> {
    use std::io::Write;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).map_err(io(path))?);
    f.write_all(b"\xEF\xBB\xBF").map_err(io(path))?;
    let mut w = csv::WriterBuilder::new().delimiter(b';').from_writer(f);
    for r in rows {
        w.write_record(r)
            .map_err(|e| Error::Export(e.to_string()))?;
    }
    w.flush().map_err(io(path))
}

mod pdf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(fmt_int(0), "0");
        assert_eq!(fmt_int(1_234_567), "1.234.567");
        assert_eq!(fmt_dec(12.345, 1), "12,3");
        assert_eq!(fmt_dur((152 * 3600 + 14 * 60) * 1000), "152h 14min");
        assert_eq!(fmt_dur(14 * 60_000 + 5000), "14min");
        assert_eq!(fmt_dur(45_000), "45s");
        assert_eq!(fmt_secs(270.0), "4min 30s");
        assert_eq!(fmt_secs(7500.0), "2h 05min");
        assert_eq!(fmt_size(3_435_973_837), "3,2 GB");
        assert_eq!(fmt_size(512), "512 B");
    }

    #[test]
    fn nice_axis_tops() {
        let tops: Vec<u64> = [0, 1, 7, 10, 11, 274, 675, 1172].map(pdf::nice).to_vec();
        assert_eq!(tops, [1, 1, 8, 10, 12, 300, 800, 1200]);
    }
}
