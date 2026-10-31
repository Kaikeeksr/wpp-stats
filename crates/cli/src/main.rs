use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;
use wpp_core::civil::Day;
use wpp_core::export::{
    fmt_dec, fmt_dur, fmt_int, fmt_secs, fmt_size, Report, KIND_LABELS, WEEKDAYS,
};
use wpp_core::index::{Index, Query, Totals};
use wpp_core::{cache, new_pool, open, tz, unzip, Kind};

const USAGE: &str = "uso: wpp-stats <pasta|.zip> [--from AAAA-MM-DD] [--to AAAA-MM-DD] [--tz IANA]\n\
                     \x20                 [--json arq] [--csv pasta] [--pdf arq] [--no-cache] [--timings]";

#[derive(Default)]
struct Args {
    dir: PathBuf,
    from: Option<Day>,
    to: Option<Day>,
    tz: Option<String>,
    json: Option<PathBuf>,
    csv: Option<PathBuf>,
    pdf: Option<PathBuf>,
    no_cache: bool,
    timings: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args::default();
    let mut dir = None;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = || it.next().ok_or(format!("{arg} precisa de um valor"));
        let date = |s: String| Day::parse(&s).ok_or(format!("data inválida: {s} (use AAAA-MM-DD)"));
        match arg.as_str() {
            "--from" => a.from = Some(date(val()?)?),
            "--to" => a.to = Some(date(val()?)?),
            "--tz" => a.tz = Some(val()?),
            "--json" => a.json = Some(val()?.into()),
            "--csv" => a.csv = Some(val()?.into()),
            "--pdf" => a.pdf = Some(val()?.into()),
            "--no-cache" => a.no_cache = true,
            "--timings" => a.timings = true,
            s if s.starts_with("--") => return Err(format!("opção desconhecida: {s}")),
            _ if dir.is_none() => dir = Some(PathBuf::from(arg)),
            _ => return Err(format!("argumento a mais: {arg}")),
        }
    }
    a.dir = dir.ok_or("falta a pasta")?;
    Ok(a)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), wpp_core::Error> {
    let pool = new_pool()?;
    let cache_dir = (!args.no_cache).then(cache::default_dir).flatten();
    let root = unzip::default_dir().unwrap_or_else(|| std::env::temp_dir().join("wpp-stats-zips"));
    let dir = unzip::folder_for(&args.dir, &root, &pool, &Default::default())?;
    let a = open(&dir, &pool, cache_dir.as_deref())?;
    let mut timings = a.timings.clone();

    let origin = tz::system_zone();
    let display = args.tz.clone().unwrap_or_else(|| origin.clone());
    let t = Instant::now();
    let ix = Index::build(&a, &origin, &display)?;
    timings.push(("índice", t.elapsed()));

    let q = Query {
        from: args.from,
        to: args.to,
        person: None,
    };
    let t = Instant::now();
    let shown = unzip::shown_path(&args.dir);
    let report = Report::new(&a, &ix, q, &shown);
    timings.push(("consulta", t.elapsed()));

    print_summary(&report);

    if let Some(p) = &args.json {
        report.write_json(p)?;
        println!("JSON: {}", p.display());
    }
    if let Some(d) = &args.csv {
        for f in report.write_csv(d)? {
            println!("CSV: {}", f.display());
        }
    }
    if let Some(p) = &args.pdf {
        let t = Instant::now();
        report.write_pdf(p)?;
        timings.push(("pdf", t.elapsed()));
        println!("PDF: {}", p.display());
    }
    if args.timings {
        println!();
        if a.from_cache {
            println!("(análise carregada do cache)");
        }
        for (name, d) in &timings {
            println!("{name:<20}{:>10.3} ms", d.as_secs_f64() * 1000.0);
        }
    }
    Ok(())
}

fn print_summary(r: &Report) {
    let s = &r.stats;
    let (from, to) = match (s.from, s.to) {
        (Some(f), Some(t)) => (f.br(), t.br()),
        _ => {
            println!("Nenhuma mensagem no período.");
            return;
        }
    };
    println!(
        "Período: {from} a {to} ({} dias) · fuso {}",
        s.days, r.ix.display
    );
    println!();

    let width = 30;
    print!("{:<width$}", "");
    for name in &s.persons {
        let short: String = name.chars().take(15).collect();
        print!("{short:>16}");
    }
    println!("{:>16}", "Total");
    let row = |label: &str, f: &dyn Fn(&Totals) -> String| {
        print!("{label:<width$}");
        for t in &s.per_person {
            print!("{:>16}", f(t));
        }
        println!("{:>16}", f(&s.total));
    };
    row("Mensagens", &|t| fmt_int(t.messages));
    for (k, label) in KIND_LABELS.iter().enumerate() {
        row(label, &|t| fmt_int(t.count[k]));
    }
    let a = Kind::Audio as usize;
    row("Bytes de texto", &|t| fmt_int(t.text_bytes));
    row("Caracteres de texto", &|t| fmt_int(t.text_chars));
    row("Tempo de áudio", &|t| fmt_dur(t.dur_ms[a]));
    row("Tempo de vídeo", &|t| {
        fmt_dur(t.dur_ms[Kind::Video as usize])
    });
    row("Tamanho de mídia", &|t| {
        fmt_size(t.media_bytes.iter().sum())
    });
    row("Mídias com legenda", &|t| fmt_int(t.captioned));
    row("Mensagens apagadas", &|t| fmt_int(t.deleted));
    row("Links", &|t| fmt_int(t.links));
    row("Emojis", &|t| fmt_int(t.emojis));
    row("Fim de semana", &|t| fmt_int(t.weekend));
    row("Madrugada (0h-4h59)", &|t| fmt_int(t.night));
    row("Mensagens por dia", &|t| fmt_dec(t.messages_per_day, 1));
    row("Minutos de áudio por dia", &|t| {
        fmt_dec(t.audio_min_per_day, 1)
    });
    print!("{:<width$}", "Mediana de resposta");
    for rsp in &s.response {
        print!("{:>16}", rsp.median_s.map_or("–".into(), fmt_secs));
    }
    println!();
    print!("{:<width$}", "Respostas consideradas");
    for rsp in &s.response {
        print!("{:>16}", fmt_int(rsp.count));
    }
    println!();

    let h = &s.highlights;
    println!();
    if let Some(d) = h.messages.first() {
        println!(
            "Dia com mais mensagens: {} ({})",
            d.day.br(),
            fmt_int(d.value)
        );
    }
    if let Some(d) = h.audio_ms.first() {
        println!(
            "Dia com mais minutos de áudio: {} ({})",
            d.day.br(),
            fmt_dur(d.value)
        );
    }
    if let (Some(hr), Some(wd)) = (h.hours.first(), h.weekdays.first()) {
        println!("Pico: {}h · {}", hr.key, WEEKDAYS[usize::from(wd.key)]);
    }
    if let Some(run) = s.longest_streak {
        println!(
            "Maior sequência: {} dias ({} a {})",
            run.days,
            run.from.br(),
            run.to.br()
        );
    }
    if let Some(run) = s.longest_gap {
        println!(
            "Maior intervalo: {} dias ({} a {})",
            run.days,
            run.from.br(),
            run.to.br()
        );
    }

    let i = r.integrity();
    println!();
    println!(
        "Integridade: {} faltantes · {} órfãos · {} sem duração · {} Outros · formato {}{}",
        i.missing.len(),
        i.orphans.len(),
        i.no_duration.len(),
        i.outros.iter().sum::<u64>(),
        i.date_order,
        if i.date_order_guessed {
            " (presumido)"
        } else {
            ""
        }
    );
    println!("SHA-256 da conversa: {}", wpp_core::hex(&r.a.chat_sha256));
}
