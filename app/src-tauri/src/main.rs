#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_window_state::StateFlags;
use wpp_core::civil::Day;
use wpp_core::export::Report;
use wpp_core::index::{Index, Query, Stats};
use wpp_core::{cache, tz, unzip, Analysis, Progress, Stage, ThreadPool};

struct Loaded {
    dir: PathBuf,
    shown: PathBuf,
    a: Analysis,
    ix: Index,
}

struct App {
    pool: ThreadPool,
    loaded: Mutex<Option<Loaded>>,
    progress: Mutex<Arc<Progress>>,
}

type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn config_path() -> Option<PathBuf> {
    wpp_core::app_dir().map(|d| d.join("config.json"))
}

#[tauri::command]
fn load_config() -> serde_json::Value {
    config_path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

#[tauri::command]
fn save_config(config: serde_json::Value) -> Res<()> {
    let p = config_path().ok_or("pasta de dados do app indisponível")?;
    std::fs::create_dir_all(p.parent().expect("has parent")).map_err(err)?;
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&config).map_err(err)?).map_err(err)?;
    std::fs::rename(tmp, p).map_err(err)
}

#[derive(Serialize)]
struct Opened {
    path: String,
    home: String,
    persons: Vec<String>,
    messages: Vec<u64>,
    first: Option<Day>,
    last: Option<Day>,
    origin: String,
    from_cache: bool,
}

#[tauri::command]
async fn open_folder(app: AppHandle, path: String) -> Res<Opened> {
    let progress = Arc::new(Progress::default());
    *app.state::<App>().progress.lock().unwrap() = progress.clone();

    let done = Arc::new(AtomicBool::new(false));
    let ticker = {
        let (app, progress, done) = (app.clone(), progress.clone(), done.clone());
        std::thread::spawn(move || {
            let mut last = u32::MAX;
            while !done.load(Relaxed) {
                let pct = (progress.fraction() * 100.0).floor() as u32;
                if pct != last {
                    let _ = app.emit("progress", pct);
                    last = pct;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        })
    };

    let job = {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let st = app.state::<App>();
            let root = unzip::default_dir().ok_or("pasta de dados do app indisponível")?;
            let dir =
                unzip::folder_for(Path::new(&path), &root, &st.pool, &progress).map_err(err)?;
            let use_cache = load_config()
                .get("cache_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let cache_dir = use_cache.then(cache::default_dir).flatten();
            let a = wpp_core::open_with(&dir, &st.pool, cache_dir.as_deref(), &progress, true)
                .map_err(err)?;
            let pending = a.extras_pending;
            let origin = tz::system_zone();
            progress.start(Stage::Index, 1);
            let ix = Index::build(&a, &origin, &origin).map_err(err)?;
            progress.finish(Stage::Index);
            let span = ix.span();
            let messages = ix
                .stats(&a, &Query::default())
                .per_person
                .iter()
                .map(|t| t.messages)
                .collect();
            let shown = unzip::shown_path(Path::new(&path));
            let opened = Opened {
                path,
                home: std::env::home_dir()
                    .map(|h| h.display().to_string())
                    .unwrap_or_default(),
                persons: ix.persons.clone(),
                messages,
                first: span.map(|s| s.0),
                last: span.map(|s| s.1),
                origin,
                from_cache: a.from_cache,
            };
            *st.loaded.lock().unwrap() = Some(Loaded {
                dir: dir.clone(),
                shown,
                a,
                ix,
            });
            if pending {
                let app = app.clone();
                std::thread::spawn(move || finish_extras(&app, &dir, cache_dir.as_deref()));
            }
            Ok(opened)
        })
    };
    let r = job.await.map_err(err)?;
    done.store(true, Relaxed);
    let _ = ticker.join();
    if r.is_ok() {
        let _ = app.emit("progress", 100u32);
    }
    r
}

fn finish_extras(app: &AppHandle, dir: &Path, cache_dir: Option<&Path>) {
    let st = app.state::<App>();
    let d = wpp_core::deferred_extras(dir, &st.pool);
    let mut guard = st.loaded.lock().unwrap();
    let Some(l) = guard.as_mut().filter(|l| l.dir == dir) else {
        return;
    };
    match d {
        Ok(d) => l.a.complete(d, dir, cache_dir),
        Err(_) => l.a.extras_pending = false,
    }
    if let Ok(ix) = Index::build(&l.a, &l.ix.origin, &l.ix.display) {
        l.ix = ix;
    }
    drop(guard);
    let _ = app.emit("extras", ());
}

#[tauri::command]
fn cancel(st: State<App>) {
    st.progress.lock().unwrap().cancel();
}

fn with_zone<T>(st: &App, zone: &str, f: impl FnOnce(&Loaded) -> Res<T>) -> Res<T> {
    let mut guard = st.loaded.lock().unwrap();
    let l = guard.as_mut().ok_or("nenhuma pasta aberta")?;
    if l.ix.display != zone {
        l.ix = Index::build(&l.a, &l.ix.origin.clone(), zone).map_err(err)?;
    }
    f(l)
}

#[tauri::command]
fn get_stats(st: State<App>, query: Query, zone: String) -> Res<Stats> {
    with_zone(&st, &zone, |l| Ok(l.ix.stats(&l.a, &query)))
}

#[tauri::command]
fn sticker(st: State<App>, id: u32) -> Res<tauri::ipc::Response> {
    let guard = st.loaded.lock().unwrap();
    let l = guard.as_ref().ok_or("nenhuma pasta aberta")?;
    let name =
        l.a.extras
            .sticker_files
            .get(id as usize)
            .ok_or("figurinha desconhecida")?;
    let bytes = std::fs::read(l.dir.join(name)).map_err(err)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[derive(Serialize)]
struct Zones {
    system: String,
    zones: Vec<(String, String)>,
}

const BRAZIL: [(&str, &str); 4] = [
    ("America/Noronha", "Fernando de Noronha"),
    ("America/Sao_Paulo", "Brasília"),
    ("America/Manaus", "Manaus"),
    ("America/Rio_Branco", "Rio Branco"),
];

#[tauri::command]
fn list_timezones() -> Zones {
    let mut zones: Vec<(String, String)> =
        BRAZIL.iter().map(|&(z, l)| (z.into(), l.into())).collect();
    zones.push(("UTC".into(), "UTC".into()));
    zones.retain(|(z, _)| tz::zone(z).is_ok());
    Zones {
        system: tz::system_zone(),
        zones,
    }
}

#[tauri::command]
fn export(
    st: State<App>,
    format: String,
    query: Query,
    zone: String,
    dest: String,
) -> Res<Vec<String>> {
    with_zone(&st, &zone, |l| {
        if l.a.extras_pending {
            return Err("ainda processando emojis e figurinhas; tente em instantes".into());
        }
        let mut r = Report::new(&l.a, &l.ix, query, &l.shown);
        r.anonymous = format == "pdf-anonimo";
        let dest = Path::new(&dest);
        match format.as_str() {
            "csv" => Ok(r
                .write_csv(dest)
                .map_err(err)?
                .iter()
                .map(|p| p.display().to_string())
                .collect()),
            "json" => r.write_json(dest).map(|_| vec![dest.display().to_string()]),
            "pdf" | "pdf-anonimo" => r.write_pdf(dest).map(|_| vec![dest.display().to_string()]),
            f => return Err(format!("formato desconhecido: {f}")),
        }
        .map_err(err)
    })
}

#[tauri::command]
fn clear_cache() -> Res<()> {
    for d in [cache::default_dir(), unzip::default_dir()]
        .into_iter()
        .flatten()
    {
        cache::clear(&d).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
fn initial_folder() -> Option<String> {
    std::env::args().nth(1)
}

fn main() {
    // WebKitGTK's DMA-BUF renderer shows a blank window on NVIDIA drivers and WSLg.
    // Set before any thread starts; an explicit value from the user wins.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    let app = App {
        pool: wpp_core::new_pool().expect("thread pool"),
        loaded: Mutex::new(None),
        progress: Mutex::new(Arc::default()),
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() - StateFlags::DECORATIONS - StateFlags::VISIBLE)
                .build(),
        )
        .manage(app)
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            open_folder,
            cancel,
            get_stats,
            sticker,
            list_timezones,
            export,
            clear_cache,
            initial_folder
        ])
        .run(tauri::generate_context!())
        .expect("tauri app");
}
