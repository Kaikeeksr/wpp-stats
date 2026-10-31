use super::*;
use crate::index::Bucket;
use krilla::color::rgb;
use krilla::geom::{Path, PathBuilder, Point};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, LineCap, LineJoin, Stroke, StrokeDash};
use krilla::surface::Surface;
use krilla::text::{Font, TextDirection};
use krilla::Document;

type Color = (u8, u8, u8);

const W: f32 = 595.0;
const HGT: f32 = 842.0;
const M: f32 = 56.0;
const CW: f32 = W - 2.0 * M;
const BOTTOM: f32 = HGT - 72.0;

const INK: Color = (0x1F, 0x1D, 0x1A);
const MUTED: Color = (0x6B, 0x67, 0x60);
const FAINT: Color = (0xA1, 0x9D, 0x95);
const RULE: Color = (0xE8, 0xE6, 0xE1);
const GRID: Color = (0xF1, 0xEF, 0xEB);
const TOTAL: Color = (0xB5, 0xB1, 0xA9);
const PERSON: [Color; 8] = [
    (0x3D, 0x84, 0xCC),
    (0xD4, 0x84, 0x2F),
    (0x2E, 0x9E, 0x7A),
    (0xB9, 0x65, 0x9A),
    (0xB8, 0x9A, 0x1E),
    (0x7B, 0x68, 0xCC),
    (0xC9, 0x5A, 0x40),
    (0x3E, 0x9C, 0x9C),
];

fn paint(c: Color) -> krilla::paint::Paint {
    rgb::Color::new(c.0, c.1, c.2).into()
}

fn person_color(p: usize) -> Color {
    PERSON.get(p).copied().unwrap_or(TOTAL)
}

#[derive(Clone, Copy)]
enum Wt {
    Regular,
    Medium,
    Semi,
    Display,
}

struct Face {
    font: Font,
    data: &'static [u8],
}

struct Fonts([Face; 4]);

impl Fonts {
    fn load() -> Result<Self, Error> {
        let face = |data: &'static [u8]| {
            Font::new(data.to_vec().into(), 0)
                .map(|font| Face { font, data })
                .ok_or_else(|| Error::Export("fonte Inter inválida".into()))
        };
        Ok(Fonts([
            face(include_bytes!("../../assets/fonts/Inter-Regular.ttf"))?,
            face(include_bytes!("../../assets/fonts/Inter-Medium.ttf"))?,
            face(include_bytes!("../../assets/fonts/Inter-SemiBold.ttf"))?,
            face(include_bytes!(
                "../../assets/fonts/InterDisplay-SemiBold.ttf"
            ))?,
        ]))
    }

    fn get(&self, w: Wt) -> &Face {
        &self.0[w as usize]
    }
}

fn width(face: &Face, size: f32, t: &str) -> f32 {
    use skrifa::instance::{LocationRef, Size};
    use skrifa::MetadataProvider;
    let Ok(f) = skrifa::FontRef::new(face.data) else {
        return t.chars().count() as f32 * size * 0.55;
    };
    let (cmap, m) = (
        f.charmap(),
        f.glyph_metrics(Size::new(size), LocationRef::default()),
    );
    t.chars()
        .map(|c| {
            cmap.map(c)
                .and_then(|g| m.advance_width(g))
                .unwrap_or(size * 0.5)
        })
        .sum()
}

fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let k = 0.552_284_8 * r;
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

struct Pen<'s, 'p> {
    s: &'s mut Surface<'p>,
    f: &'s Fonts,
}

impl Pen<'_, '_> {
    fn text(&mut self, x: f32, y: f32, size: f32, color: Color, w: Wt, t: &str) {
        self.s.set_fill(Some(Fill {
            paint: paint(color),
            ..Default::default()
        }));
        self.s.draw_text(
            Point::from_xy(x, y),
            self.f.get(w).font.clone(),
            size,
            t,
            false,
            TextDirection::Auto,
        );
    }

    fn width(&self, size: f32, w: Wt, t: &str) -> f32 {
        width(self.f.get(w), size, t)
    }

    fn text_r(&mut self, x: f32, y: f32, size: f32, color: Color, w: Wt, t: &str) {
        let tw = self.width(size, w, t);
        self.text(x - tw, y, size, color, w, t);
    }

    fn fit(&self, size: f32, w: Wt, t: &str, max: f32) -> String {
        if self.width(size, w, t) <= max {
            return t.to_string();
        }
        let mut s: String = t.to_string();
        while !s.is_empty() && self.width(size, w, &format!("{s}…")) > max {
            s.pop();
        }
        format!("{}…", s.trim_end())
    }

    fn fill(&mut self, path: Option<Path>, color: Color, opacity: f32) {
        if let Some(path) = path {
            self.s.set_stroke(None);
            self.s.set_fill(Some(Fill {
                paint: paint(color),
                opacity: NormalizedF32::new(opacity).unwrap_or(NormalizedF32::ONE),
                ..Default::default()
            }));
            self.s.draw_path(&path);
        }
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        if w > 0.05 && h > 0.05 {
            self.fill(rounded(x, y, w, h, r), color, 1.0);
        }
    }

    fn dot(&mut self, x: f32, y: f32, r: f32, color: Color) {
        self.fill(rounded(x - r, y - r, 2.0 * r, 2.0 * r, r), color, 1.0);
    }

    fn line(&mut self, pts: &[(f32, f32)], color: Color, width: f32, dash: bool) {
        let mut pb = PathBuilder::new();
        for (i, &(x, y)) in pts.iter().enumerate() {
            if i == 0 {
                pb.move_to(x, y);
            } else {
                pb.line_to(x, y);
            }
        }
        if let Some(path) = pb.finish() {
            self.s.set_fill(None);
            self.s.set_stroke(Some(Stroke {
                paint: paint(color),
                width,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                dash: dash.then(|| StrokeDash {
                    array: vec![2.5, 3.0],
                    offset: 0.0,
                }),
                ..Default::default()
            }));
            self.s.draw_path(&path);
            self.s.set_stroke(None);
        }
    }

    fn area(&mut self, pts: &[(f32, f32)], base: f32, color: Color, opacity: f32) {
        let (Some(&(x0, _)), Some(&(x1, _))) = (pts.first(), pts.last()) else {
            return;
        };
        let mut pb = PathBuilder::new();
        pb.move_to(x0, base);
        for &(x, y) in pts {
            pb.line_to(x, y);
        }
        pb.line_to(x1, base);
        pb.close();
        self.fill(pb.finish(), color, opacity);
    }

    fn hline(&mut self, x: f32, y: f32, w: f32, color: Color) {
        self.line(&[(x, y), (x + w, y)], color, 0.6, false);
    }

    fn heading(&mut self, y: f32, t: &str) -> f32 {
        self.text(M, y, 11.0, INK, Wt::Semi, t);
        y + 22.0
    }

    fn who(&mut self, x: f32, y: f32, name: &str, color: Color, max: f32) -> f32 {
        self.dot(x + 3.0, y - 3.0, 3.0, color);
        let n = self.fit(8.5, Wt::Regular, name, max - 11.0);
        self.text(x + 11.0, y, 8.5, MUTED, Wt::Regular, &n);
        11.0 + self.width(8.5, Wt::Regular, &n)
    }
}

type PageFn = fn(&mut Pen, &Report);

pub fn render(r: &Report) -> Result<Vec<u8>, Error> {
    let fonts = Fonts::load()?;
    let mut doc = Document::new();
    let mut pages: Vec<PageFn> = vec![page_cover];
    if r.stats.total.messages > 0 {
        pages.push(page_summary);
        if group(r) {
            pages.push(page_participants);
        }
        pages.push(page_rhythm);
    }
    pages.push(page_integrity);
    let n = pages.len();
    for (i, draw) in pages.iter().enumerate() {
        let mut page = doc.start_page_with(PageSettings::from_wh(W, HGT).expect("A4"));
        let mut s = page.surface();
        let mut pen = Pen {
            s: &mut s,
            f: &fonts,
        };
        draw(&mut pen, r);
        footer(&mut pen, r, i + 1, n);
        s.finish();
        page.finish();
    }
    doc.finish().map_err(|e| Error::Export(format!("{e:?}")))
}

fn footer(p: &mut Pen, r: &Report, page: usize, pages: usize) {
    let y = HGT - 44.0;
    p.hline(M, y - 12.0, CW, RULE);
    p.text(
        M,
        y,
        6.5,
        FAINT,
        Wt::Regular,
        &format!(
            "WPP Stats {APP_VERSION} · análise em {} · fuso de origem {} · fuso de exibição {} · período {}",
            r.analyzed_at(),
            r.ix.origin,
            r.ix.display,
            r.period()
        ),
    );
    p.text(
        M,
        y + 10.0,
        6.5,
        FAINT,
        Wt::Regular,
        &format!("SHA-256 do arquivo da conversa  {}", hex(&r.a.chat_sha256)),
    );
    p.text_r(
        W - M,
        y + 10.0,
        6.5,
        FAINT,
        Wt::Medium,
        &format!("{page} / {pages}"),
    );
}

fn running(p: &mut Pen, r: &Report, title: &str) -> f32 {
    let name = p.fit(8.0, Wt::Regular, &r.folder_name(), CW / 2.0);
    p.text(M, M, 8.0, FAINT, Wt::Regular, &name);
    p.text_r(W - M, M, 8.0, FAINT, Wt::Regular, &r.period());
    p.text(M, M + 36.0, 20.0, INK, Wt::Display, title);
    M + 68.0
}

fn group(r: &Report) -> bool {
    r.sel().len() > 2
}

fn n(v: u64, one: &str, many: &str) -> String {
    format!("{} {}", fmt_int(v), if v == 1 { one } else { many })
}

fn msgs(v: u64) -> String {
    n(v, "mensagem", "mensagens")
}

fn weekday_name(d: Day) -> String {
    WEEKDAYS[crate::civil::weekday(d.0)].to_lowercase()
}

fn pct(part: u64, whole: u64) -> String {
    format!("{}%", fmt_dec(100.0 * part as f64 / whole.max(1) as f64, 0))
}

const FILES: [Kind; 6] = [
    Kind::Foto,
    Kind::Video,
    Kind::Figurinha,
    Kind::Gif,
    Kind::Documento,
    Kind::Audio,
];

fn files(t: &Totals) -> u64 {
    FILES.iter().map(|&k| t.count[k as usize]).sum()
}

fn page_cover(p: &mut Pen, r: &Report) {
    let s = &r.stats;
    let kicker = if r.anonymous {
        "WPP STATS · RELATÓRIO ANÔNIMO"
    } else {
        "WPP STATS · RELATÓRIO"
    };
    p.text(M, M, 8.0, FAINT, Wt::Medium, kicker);
    let title = p.fit(30.0, Wt::Display, &r.folder_name(), CW);
    p.text(M, M + 52.0, 30.0, INK, Wt::Display, &title);

    let mut y = M + 78.0;
    let sel = r.sel();
    if sel.len() > 2 {
        p.text(
            M,
            y,
            9.5,
            MUTED,
            Wt::Regular,
            &format!("{} participantes", sel.len()),
        );
    } else {
        let mut x = M;
        for &i in &sel {
            x += p.who(x, y, &r.person(i), person_color(i), CW / 2.0) + 16.0;
        }
    }
    y += 16.0;
    p.text(
        M,
        y,
        9.5,
        MUTED,
        Wt::Regular,
        &format!(
            "{} · {} · fuso {}",
            r.period(),
            n(u64::from(s.days), "dia", "dias"),
            r.ix.display
        ),
    );
    y += 28.0;
    p.hline(M, y, CW, RULE);
    y += 32.0;

    if s.total.messages == 0 {
        p.text(M, y, 11.0, INK, Wt::Regular, "Nenhuma mensagem no período.");
        return;
    }

    let a = Kind::Audio as usize;
    type Card<'a> = (&'a str, fn(&Totals) -> u64, fn(u64) -> String, String);
    let cards: [Card; 3] = [
        (
            "Mensagens",
            |t| t.messages,
            fmt_int,
            format!("{} por dia", fmt_dec(s.total.messages_per_day, 1)),
        ),
        (
            "Tempo de áudio",
            |t| t.dur_ms[Kind::Audio as usize],
            fmt_dur,
            n(s.total.count[a], "áudio", "áudios"),
        ),
        (
            "Mídias",
            |t| t.media_bytes.iter().sum(),
            fmt_size,
            n(files(&s.total), "arquivo", "arquivos"),
        ),
    ];
    let gap = 24.0;
    let cw = (CW - 2.0 * gap) / 3.0;
    let split = (2..=2).contains(&sel.len());
    for (k, (label, f, fmt, sub)) in cards.iter().enumerate() {
        let x = M + k as f32 * (cw + gap);
        let all = f(&s.total);
        p.text(x, y, 8.5, MUTED, Wt::Regular, label);
        p.text(x, y + 28.0, 24.0, INK, Wt::Display, &fmt(all));
        p.text(x, y + 43.0, 8.0, FAINT, Wt::Regular, sub);
        if split {
            let mut bx = x;
            let room = cw - 3.0 * (sel.len() - 1) as f32;
            for &i in &sel {
                let bw = room * f(&s.per_person[i]) as f32 / all.max(1) as f32;
                p.rect(bx, y + 56.0, bw, 4.0, 2.0, person_color(i));
                bx += bw + 3.0;
            }
            for (j, &i) in sel.iter().enumerate() {
                let ry = y + 78.0 + j as f32 * 14.0;
                let v = f(&s.per_person[i]);
                p.who(x, ry, &r.person(i), person_color(i), cw * 0.45);
                p.text_r(x + cw - 26.0, ry, 8.5, INK, Wt::Medium, &fmt(v));
                p.text_r(x + cw, ry, 8.0, FAINT, Wt::Regular, &pct(v, all));
            }
        }
    }
    y += if split { 130.0 } else { 80.0 };

    let series: Vec<(usize, Color)> = if group(r) {
        Vec::new()
    } else {
        sel.iter().map(|&i| (i, person_color(i))).collect()
    };

    let (labels, totals, unit) = trend_series(&s.timeline);
    p.heading(y, &format!("Mensagens por {unit}"));
    let avg = totals.iter().sum::<u64>() as f64 / totals.len().max(1) as f64;
    legend(
        p,
        y,
        &[Legend::Dash(&format!("média de {}", fmt_dec(avg, 1)))],
    );
    trend_chart(p, M, y + 22.0, CW, 140.0, &totals, &labels);
    y += 214.0;

    p.heading(y, "Mensagens acumuladas");
    let pr = &s.progression;
    let lines: Vec<(&[u64], Color)> = if series.is_empty() {
        vec![(&pr.total[..], INK)]
    } else {
        series
            .iter()
            .map(|&(i, c)| (&pr.per_person[i][..], c))
            .collect()
    };
    if series.len() > 1 {
        let names: Vec<String> = series.iter().map(|&(i, _)| r.person(i)).collect();
        let items: Vec<Legend> = series
            .iter()
            .zip(&names)
            .map(|(&(_, c), n)| Legend::Dot(n, c))
            .collect();
        legend(p, y, &items);
    }
    line_chart(p, M, y + 22.0, CW, 140.0, &lines, &pr.labels);
}

fn trend_series(tl: &crate::index::Series) -> (Vec<Day>, Vec<u64>, &'static str) {
    match tl.bucket {
        Bucket::Dia if tl.labels.len() > 90 => {
            let (mut labels, mut totals) = (Vec::new(), Vec::<u64>::new());
            for (d, &v) in tl.labels.iter().zip(&tl.total) {
                let wk = Day(d.0 - crate::civil::weekday(d.0) as i64);
                if labels.last() != Some(&wk) {
                    labels.push(wk);
                    totals.push(0);
                }
                *totals.last_mut().expect("pushed") += v;
            }
            (labels, totals, "semana")
        }
        b => (
            tl.labels.clone(),
            tl.total.clone(),
            match b {
                Bucket::Dia => "dia",
                Bucket::Semana => "semana",
                Bucket::Mes => "mês",
            },
        ),
    }
}

type Cell<'a> = Box<dyn Fn(&Totals) -> String + 'a>;

enum Row<'a> {
    Section(&'a str),
    Line(&'a str, Cell<'a>),
}

fn line<'a>(label: &'a str, f: impl Fn(&Totals) -> String + 'a) -> Row<'a> {
    Row::Line(label, Box::new(f))
}

fn page_summary(p: &mut Pen, r: &Report) {
    let s = &r.stats;
    let sel = if group(r) { Vec::new() } else { r.sel() };
    let mut y = running(
        p,
        r,
        if sel.is_empty() {
            "Resumo"
        } else {
            "Resumo por pessoa"
        },
    );

    let col_w = 88.0;
    let right = W - M;
    let ncols = sel.len() + 1;
    let col_x = |j: usize| right - (ncols - 1 - j) as f32 * col_w;
    for (j, &i) in sel.iter().enumerate() {
        let name = p.fit(8.5, Wt::Medium, &r.person(i), col_w - 16.0);
        let w = p.width(8.5, Wt::Medium, &name);
        p.dot(col_x(j) - w - 7.0, y - 3.0, 3.0, person_color(i));
        p.text_r(col_x(j), y, 8.5, INK, Wt::Medium, &name);
    }
    p.text_r(col_x(ncols - 1), y, 8.5, MUTED, Wt::Medium, "Total");
    y += 8.0;

    let a = Kind::Audio as usize;
    let count_size = |k: Kind| {
        let k = k as usize;
        move |t: &Totals| {
            if t.count[k] == 0 {
                "0".into()
            } else {
                format!("{} · {}", fmt_int(t.count[k]), fmt_size(t.media_bytes[k]))
            }
        }
    };
    let rows: Vec<Row> = vec![
        Row::Section("Mensagens"),
        line("Mensagens (todos os tipos)", |t| fmt_int(t.messages)),
        line("Mensagens por dia", |t| fmt_dec(t.messages_per_day, 1)),
        line("No fim de semana", |t| fmt_int(t.weekend)),
        line("Na madrugada (0h a 4h59)", |t| fmt_int(t.night)),
        line("Apagadas", |t| fmt_int(t.deleted)),
        Row::Section("Texto"),
        line("Mensagens de texto", |t| {
            fmt_int(t.count[Kind::Texto as usize])
        }),
        line("Caracteres", |t| fmt_int(t.text_chars)),
        line("Bytes (UTF-8)", |t| fmt_int(t.text_bytes)),
        line("Links", |t| fmt_int(t.links)),
        line("Emojis", |t| fmt_int(t.emojis)),
        Row::Section("Áudio"),
        line("Áudios", move |t| fmt_int(t.count[a])),
        line("Tempo total", move |t| fmt_dur(t.dur_ms[a])),
        line("Minutos por dia", |t| fmt_dec(t.audio_min_per_day, 1)),
        Row::Section("Arquivos · quantidade e tamanho"),
        line("Fotos", count_size(Kind::Foto)),
        line("Vídeos", count_size(Kind::Video)),
        line("Figurinhas", count_size(Kind::Figurinha)),
        line("GIFs", count_size(Kind::Gif)),
        line("Documentos", count_size(Kind::Documento)),
        line("Áudios", count_size(Kind::Audio)),
        line("Total", |t| {
            format!(
                "{} · {}",
                fmt_int(files(t)),
                fmt_size(t.media_bytes.iter().sum())
            )
        }),
        Row::Section("Visualização única · sem arquivo"),
        line("Fotos", |t| fmt_int(t.count[Kind::FotoUnica as usize])),
        line("Vídeos", |t| fmt_int(t.count[Kind::VideoUnico as usize])),
        line("Áudios", |t| fmt_int(t.count[Kind::AudioUnico as usize])),
        Row::Section("Outros"),
        line("Tempo de vídeo", |t| {
            fmt_dur(t.dur_ms[Kind::Video as usize])
        }),
        line("Mídias com legenda", |t| fmt_int(t.captioned)),
        line("Linhas não reconhecidas", |t| {
            fmt_int(t.count[Kind::Outros as usize])
        }),
    ];
    for row in &rows {
        match row {
            Row::Section(t) => {
                y += 18.0;
                p.text(M, y, 7.5, FAINT, Wt::Medium, &t.to_uppercase());
                y += 6.0;
            }
            Row::Line(label, f) => {
                p.hline(M, y, CW, GRID);
                y += 10.5;
                p.text(M, y, 8.5, MUTED, Wt::Regular, label);
                let vals = sel
                    .iter()
                    .map(|&i| f(&s.per_person[i]))
                    .chain([f(&s.total)]);
                for (j, v) in vals.enumerate() {
                    let (color, w) = if j + 1 == ncols {
                        (MUTED, Wt::Regular)
                    } else {
                        (INK, Wt::Medium)
                    };
                    p.text_r(col_x(j), y, 8.5, color, w, &v);
                }
                y += 4.5;
            }
        }
    }

    if !sel.is_empty() {
        response_block(p, r, &sel, y + 30.0);
    }
}

fn secs(x: Option<f64>) -> String {
    x.map_or("–".into(), fmt_secs)
}

fn response_block(p: &mut Pen, r: &Report, sel: &[usize], y: f32) {
    let s = &r.stats;
    let mut y = p.heading(y, "Tempo de resposta");
    let col_w = 96.0;
    let x_of = |j: usize| W - M - (3 - j) as f32 * col_w;
    for (j, label) in [
        "Em 50% das vezes",
        "Em 75% das vezes",
        "Em 90% das vezes",
        "Respostas",
    ]
    .iter()
    .enumerate()
    {
        p.text_r(x_of(j), y, 8.0, MUTED, Wt::Medium, label);
    }
    y += 8.0;
    for &i in sel {
        let rsp = &s.response[i];
        p.hline(M, y, CW, GRID);
        y += 13.0;
        p.who(M, y, &r.person(i), person_color(i), x_of(0) - col_w - M);
        let vals = [
            secs(rsp.median_s),
            secs(rsp.p75_s),
            secs(rsp.p90_s),
            fmt_int(rsp.count),
        ];
        for (j, v) in vals.iter().enumerate() {
            let w = if j == 3 { Wt::Regular } else { Wt::Semi };
            p.text_r(x_of(j), y, 8.5, INK, w, v);
        }
        y += 5.5;
    }
    p.text(
        M,
        y + 16.0,
        7.5,
        FAINT,
        Wt::Regular,
        "Tempo máximo em que a pessoa respondeu nessa parcela das vezes, contado da última mensagem da outra pessoa, sem limite.",
    );
}

fn page_participants(p: &mut Pen, r: &Report) {
    let s = &r.stats;
    let mut y = running(p, r, "Participantes");
    let mut sel = r.sel();
    sel.sort_by(|&a, &b| {
        s.per_person[b]
            .messages
            .cmp(&s.per_person[a].messages)
            .then(a.cmp(&b))
    });
    let a = Kind::Audio as usize;
    let once = |t: &Totals| {
        [Kind::FotoUnica, Kind::VideoUnico, Kind::AudioUnico]
            .iter()
            .map(|&k| t.count[k as usize])
            .sum::<u64>()
    };
    let all = s.total.messages;
    type Col<'a> = (&'a str, Box<dyn Fn(&Totals, Option<usize>) -> String + 'a>);
    let cols: Vec<Col> = vec![
        ("Mensagens", Box::new(|t, _| fmt_int(t.messages))),
        ("%", Box::new(move |t, _| pct(t.messages, all))),
        (
            "Texto",
            Box::new(|t, _| fmt_int(t.count[Kind::Texto as usize])),
        ),
        ("Áudios", Box::new(move |t, _| fmt_int(t.count[a]))),
        ("Duração", Box::new(move |t, _| fmt_dur(t.dur_ms[a]))),
        ("Arquivos", Box::new(|t, _| fmt_int(files(t)))),
        ("Vis. única", Box::new(move |t, _| fmt_int(once(t)))),
        (
            "Resposta",
            Box::new(|_, i| secs(i.and_then(|i| s.response[i].median_s))),
        ),
    ];
    let name_w = 104.0;
    let col_w = (CW - name_w) / cols.len() as f32;
    let x_of = |j: usize| M + name_w + (j + 1) as f32 * col_w;
    for (j, (label, _)) in cols.iter().enumerate() {
        p.text_r(x_of(j), y, 8.0, MUTED, Wt::Medium, label);
    }
    y += 8.0;
    let row = |p: &mut Pen, y: f32, i: Option<usize>, t: &Totals| {
        match i {
            Some(i) => {
                p.who(M, y, &r.person(i), person_color(i), name_w - 8.0);
            }
            None => p.text(M, y, 8.5, INK, Wt::Semi, "Total"),
        }
        let w = if i.is_none() { Wt::Semi } else { Wt::Regular };
        for (j, (_, f)) in cols.iter().enumerate() {
            p.text_r(x_of(j), y, 8.5, INK, w, &f(t, i));
        }
    };
    for (k, &i) in sel.iter().enumerate() {
        if y > BOTTOM - 60.0 {
            y += 14.0;
            p.text(
                M,
                y,
                8.5,
                MUTED,
                Wt::Regular,
                &format!("… e mais {} participantes", sel.len() - k),
            );
            break;
        }
        p.hline(M, y, CW, GRID);
        y += 13.0;
        row(p, y, Some(i), &s.per_person[i]);
        y += 5.0;
    }
    p.hline(M, y, CW, RULE);
    y += 14.0;
    row(p, y, None, &s.total);
    p.text(
        M,
        y + 26.0,
        7.5,
        FAINT,
        Wt::Regular,
        "Resposta: mediana do tempo até a pessoa falar depois de outra. Em grupo, não indica a quem ela respondeu.",
    );
}

fn page_rhythm(p: &mut Pen, r: &Report) {
    let s = &r.stats;
    let mut y = running(p, r, "Ritmo e destaques");

    p.heading(y, "Mensagens por dia da semana e hora");
    heat_scale(p, y);
    y = heatmap(p, M, y + 18.0, CW, &s.heatmap_total) + 34.0;

    y = p.heading(y, "Destaques");
    let h = &s.highlights;
    let day = |l: &[DayValue], f: &dyn Fn(u64) -> String| {
        l.first().map_or(("–".into(), String::new()), |d| {
            (
                d.day.br(),
                format!("{} · {}", weekday_name(d.day), f(d.value)),
            )
        })
    };
    let run = |x: &Option<Run>| {
        x.map_or(("–".into(), String::new()), |r| {
            (
                n(u64::from(r.days), "dia", "dias"),
                format!("{} a {}", r.from.br(), r.to.br()),
            )
        })
    };
    let items = [
        ("Dia com mais mensagens", day(&h.messages, &msgs)),
        (
            "Dia com mais áudios",
            day(&h.audios, &|v| n(v, "áudio", "áudios")),
        ),
        ("Dia com mais tempo de áudio", day(&h.audio_ms, &fmt_dur)),
        (
            "Dia com mais mídias",
            day(&h.media, &|v| n(v, "mídia", "mídias")),
        ),
        (
            "Horário preferido",
            h.hours.first().map_or(("–".into(), String::new()), |x| {
                (format!("{}h às {}h", x.key, x.key + 1), msgs(x.value))
            }),
        ),
        (
            "Dia preferido",
            h.weekdays.first().map_or(("–".into(), String::new()), |x| {
                (WEEKDAYS[usize::from(x.key)].to_string(), msgs(x.value))
            }),
        ),
        ("Dias seguidos com mensagens", run(&s.longest_streak)),
        ("Maior intervalo sem mensagens", run(&s.longest_gap)),
    ];
    let col = CW / 2.0;
    for (k, (label, (value, sub))) in items.iter().enumerate() {
        let x = M + (k % 2) as f32 * col;
        let iy = y + (k / 2) as f32 * 44.0;
        if k % 2 == 0 {
            p.hline(M, iy - 10.0, CW, GRID);
        }
        p.text(x, iy, 8.0, MUTED, Wt::Regular, label);
        p.text(x, iy + 15.0, 12.0, INK, Wt::Semi, value);
        p.text(x, iy + 26.0, 8.0, FAINT, Wt::Regular, sub);
    }
    y += 4.0 * 44.0 + 22.0;

    if !h.messages.is_empty() && y + 40.0 < BOTTOM {
        y = p.heading(y, "Top 10 dias por mensagens");
        let max = h.messages[0].value.max(1) as f32;
        for (k, d) in h.messages.iter().enumerate() {
            if y > BOTTOM {
                break;
            }
            p.text_r(M + 12.0, y, 8.5, FAINT, Wt::Regular, &format!("{}", k + 1));
            p.text(M + 22.0, y, 8.5, INK, Wt::Medium, &d.day.br());
            p.text(M + 76.0, y, 8.5, FAINT, Wt::Regular, &weekday_name(d.day));
            p.rect(
                M + 140.0,
                y - 6.0,
                250.0 * d.value as f32 / max,
                6.0,
                3.0,
                TOTAL,
            );
            p.text_r(W - M, y, 8.5, INK, Wt::Regular, &msgs(d.value));
            y += 16.0;
        }
    }
}

fn page_integrity(p: &mut Pen, r: &Report) {
    let mut y = running(p, r, "Integridade");
    let i = r.integrity();
    p.text(
        M,
        y - 14.0,
        8.5,
        MUTED,
        Wt::Regular,
        "Os números desta página cobrem a conversa inteira, não só o período.",
    );
    y += 12.0;
    let per = |v: &[u64]| -> String {
        v.iter()
            .enumerate()
            .map(|(k, x)| format!("{}: {}", r.person(k), fmt_int(*x)))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let ignored: Vec<u64> = i.ignored.iter().map(|&x| u64::from(x)).collect();
    let order = if i.date_order_guessed {
        format!("{} (presumido: arquivo ambíguo)", i.date_order)
    } else {
        i.date_order.to_string()
    };
    let lines = [
        (
            "Mídias citadas e ausentes na pasta",
            fmt_int(i.missing.len() as u64),
        ),
        (
            "Arquivos na pasta e não citados",
            fmt_int(i.orphans.len() as u64),
        ),
        ("Mídias sem duração", fmt_int(i.no_duration.len() as u64)),
        ("Linhas não reconhecidas (Outros)", per(&i.outros)),
        ("Mídias com legenda", fmt_int(i.captioned)),
        ("Ignoradas (sistema e apagadas)", per(&ignored)),
        (
            "Horários ambíguos (horário de verão)",
            fmt_int(u64::from(i.dst_ambiguous)),
        ),
        (
            "Horários inexistentes (horário de verão)",
            fmt_int(u64::from(i.dst_nonexistent)),
        ),
        ("Formato de data", order),
        ("Datas inválidas", fmt_int(u64::from(i.bad_dates))),
        ("Linhas inválidas", fmt_int(u64::from(i.invalid_lines))),
        ("Linhas sem autor", fmt_int(u64::from(i.no_author))),
    ];
    for (label, value) in lines {
        p.hline(M, y, CW, GRID);
        y += 13.0;
        p.text(M, y, 8.5, MUTED, Wt::Regular, label);
        let v = p.fit(8.5, Wt::Medium, &value, CW - 210.0);
        p.text(M + 210.0, y, 8.5, INK, Wt::Medium, &v);
        y += 5.5;
    }
    y += 30.0;

    if r.anonymous {
        p.text(
            M,
            y,
            8.0,
            FAINT,
            Wt::Regular,
            "Nomes de arquivos omitidos no relatório anônimo.",
        );
        return;
    }
    for (heading, names) in [
        ("Mídias citadas e ausentes", i.missing),
        ("Arquivos não citados", i.orphans),
        ("Mídias sem duração", i.no_duration),
    ] {
        if names.is_empty() || y > BOTTOM - 40.0 {
            continue;
        }
        y = p.heading(y, heading);
        for (k, name) in names.iter().enumerate() {
            if y > BOTTOM {
                p.text(
                    M,
                    y,
                    8.0,
                    MUTED,
                    Wt::Regular,
                    &format!("… e mais {}", names.len() - k),
                );
                y += 12.0;
                break;
            }
            let short = p.fit(8.0, Wt::Regular, name, CW);
            p.text(M, y, 8.0, INK, Wt::Regular, &short);
            y += 11.5;
        }
        y += 14.0;
    }
}

enum Legend<'a> {
    Dot(&'a str, Color),
    Dash(&'a str),
}

fn legend(p: &mut Pen, y: f32, items: &[Legend]) {
    let mut x = W - M;
    for it in items.iter().rev() {
        let (Legend::Dot(l, _) | Legend::Dash(l)) = it;
        let w = p.width(8.0, Wt::Regular, l);
        p.text(x - w, y, 8.0, MUTED, Wt::Regular, l);
        x -= w + 5.0;
        match it {
            Legend::Dot(_, c) => {
                p.dot(x - 3.0, y - 3.0, 3.0, *c);
                x -= 6.0;
            }
            Legend::Dash(_) => {
                p.line(&[(x - 12.0, y - 3.0), (x, y - 3.0)], FAINT, 0.9, true);
                x -= 12.0;
            }
        }
        x -= 14.0;
    }
}

fn heat_scale(p: &mut Pen, y: f32) {
    let mut x = W - M;
    let w = p.width(7.5, Wt::Regular, "mais");
    p.text(x - w, y, 7.5, FAINT, Wt::Regular, "mais");
    x -= w + 4.0;
    for t in [1.0, 0.66, 0.33, 0.0] {
        x -= 9.0;
        p.rect(x, y - 7.0, 8.0, 8.0, 2.0, heat(t));
    }
    let w = p.width(7.5, Wt::Regular, "menos");
    p.text(x - 4.0 - w, y, 7.5, FAINT, Wt::Regular, "menos");
}

pub(super) fn nice(v: u64) -> u64 {
    let v = v.max(1);
    let mut pow = 1u64;
    while pow * 10 < v {
        pow *= 10;
    }
    [10, 12, 15, 20, 25, 30, 40, 50, 60, 80, 100]
        .iter()
        .map(|&m| m * pow / 10)
        .find(|&c| c >= v)
        .unwrap_or(10 * pow)
}

const AXIS: f32 = 28.0;

fn grid(p: &mut Pen, x: f32, y: f32, w: f32, h: f32, max: u64) -> f32 {
    let top = nice(max);
    for k in 0..=2u64 {
        let gy = y + h - h * k as f32 / 2.0;
        p.hline(x + AXIS, gy, w - AXIS, if k == 0 { RULE } else { GRID });
        p.text_r(
            x + AXIS - 6.0,
            gy + 2.5,
            7.0,
            FAINT,
            Wt::Regular,
            &fmt_int(top * k / 2),
        );
    }
    top as f32
}

fn axis_labels(p: &mut Pen, x: f32, y: f32, w: f32, labels: &[Day]) {
    if let (Some(a), Some(b)) = (labels.first(), labels.last()) {
        p.text(x, y, 7.0, FAINT, Wt::Regular, &a.br());
        p.text_r(x + w, y, 7.0, FAINT, Wt::Regular, &b.br());
    }
}

fn points(v: &[u64], x: f32, y: f32, w: f32, h: f32, top: f32) -> Vec<(f32, f32)> {
    let n = v.len().max(2) - 1;
    v.iter()
        .enumerate()
        .map(|(i, &val)| (x + w * i as f32 / n as f32, y + h - h * val as f32 / top))
        .collect()
}

fn trend_chart(p: &mut Pen, x: f32, y: f32, w: f32, h: f32, v: &[u64], labels: &[Day]) {
    let end_room = 24.0;
    let top = grid(p, x, y, w, h, v.iter().copied().max().unwrap_or(0));
    let (px, pw) = (x + AXIS, w - AXIS - end_room);
    let pts = points(v, px, y, pw, h, top);
    p.area(&pts, y + h, INK, 0.05);
    let avg = v.iter().sum::<u64>() as f32 / v.len().max(1) as f32;
    let ay = y + h - h * avg / top;
    p.line(&[(px, ay), (px + pw, ay)], FAINT, 0.8, true);
    p.line(&pts, INK, 1.4, false);
    let hi = (0..v.len()).max_by_key(|&i| (v[i], usize::MAX - i));
    let last = v.len().checked_sub(1);
    for (i, strong) in hi
        .into_iter()
        .map(|i| (i, false))
        .chain(last.filter(|&l| Some(l) != hi).map(|l| (l, true)))
    {
        let (cx, cy) = pts[i];
        p.dot(cx, cy, 2.6, INK);
        let label = fmt_int(v[i]);
        if strong {
            p.text(cx + 6.0, cy + 3.0, 8.0, INK, Wt::Semi, &label);
        } else {
            let lw = p.width(8.0, Wt::Semi, &label);
            p.text(cx - lw / 2.0, cy - 6.0, 8.0, MUTED, Wt::Semi, &label);
        }
    }
    axis_labels(p, px, y + h + 12.0, pw, labels);
}

fn line_chart(
    p: &mut Pen,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    series: &[(&[u64], Color)],
    labels: &[Day],
) {
    let end_room = 34.0;
    let max = series
        .iter()
        .flat_map(|s| s.0.iter())
        .copied()
        .max()
        .unwrap_or(0);
    let top = grid(p, x, y, w - end_room, h, max);
    let (px, pw) = (x + AXIS, w - AXIS - end_room);
    let mut ends = Vec::new();
    for &(v, color) in series {
        let pts = points(v, px, y, pw, h, top);
        p.area(&pts, y + h, color, 0.06);
        p.line(&pts, color, 1.4, false);
        if let (Some(&(ex, ey)), Some(&last)) = (pts.last(), v.last()) {
            ends.push((ey, ex, last, color));
        }
    }
    ends.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut prev = f32::MIN;
    for (ey, ex, val, color) in ends {
        let ly = (ey + 3.0).max(prev + 10.0);
        p.text(ex + 5.0, ly, 8.0, color, Wt::Semi, &fmt_int(val));
        prev = ly;
    }
    axis_labels(p, px, y + h + 12.0, pw, labels);
}

fn heat(t: f32) -> Color {
    let lerp = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
    (lerp(0xF1, 0x3A), lerp(0xF0, 0x37), lerp(0xEC, 0x33))
}

fn heatmap(p: &mut Pen, x: f32, y: f32, w: f32, cells: &[[u64; 24]; 7]) -> f32 {
    const DAYS: [&str; 7] = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
    let max = (cells.iter().flatten().copied().max().unwrap_or(0).max(1) as f32).sqrt();
    let (lw, ch) = (32.0, 16.0);
    let cw = (w - lw) / 24.0;
    for (d, row) in cells.iter().enumerate() {
        let cy = y + d as f32 * ch;
        p.text(x, cy + 10.5, 7.5, FAINT, Wt::Regular, DAYS[d]);
        for (hr, &v) in row.iter().enumerate() {
            let t = (v as f32).sqrt() / max;
            p.rect(
                x + lw + hr as f32 * cw,
                cy,
                cw - 2.0,
                ch - 2.0,
                2.5,
                heat(t),
            );
        }
    }
    let ly = y + 7.0 * ch + 10.0;
    for hr in (0..24).step_by(3) {
        p.text(
            x + lw + hr as f32 * cw,
            ly,
            7.0,
            FAINT,
            Wt::Regular,
            &format!("{hr}h"),
        );
    }
    ly
}
