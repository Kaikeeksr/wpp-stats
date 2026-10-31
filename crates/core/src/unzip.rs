use crate::{hex, io, mtime_ns, Error, Progress, Stage};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn default_dir() -> Option<PathBuf> {
    crate::app_dir().map(|d| d.join("zips"))
}

pub fn is_zip(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

pub fn shown_path(path: &Path) -> PathBuf {
    if is_zip(path) {
        path.with_extension("")
    } else {
        path.to_path_buf()
    }
}

pub fn folder_for(
    path: &Path,
    root: &Path,
    pool: &rayon::ThreadPool,
    progress: &Progress,
) -> Result<PathBuf, Error> {
    if !is_zip(path) {
        return Ok(path.to_path_buf());
    }
    let md = std::fs::metadata(path).map_err(io(path))?;
    let id = Sha256::digest(format!("{}|{}|{}", path.display(), md.len(), mtime_ns(&md)));
    let name = hex(&id[..8]);
    let dest = root.join(&name);
    if dest.is_dir() {
        return Ok(dest);
    }
    let part = root.join(name + ".part");
    let _ = std::fs::remove_dir_all(&part);
    std::fs::create_dir_all(&part).map_err(io(path))?;
    extract(path, &part, pool, progress)?;
    std::fs::rename(&part, &dest).map_err(io(path))?;
    Ok(dest)
}

fn extract(
    zip: &Path,
    dest: &Path,
    pool: &rayon::ThreadPool,
    progress: &Progress,
) -> Result<(), Error> {
    let bad = |e: zip::result::ZipError| io(zip)(std::io::Error::other(e));
    let open = || -> Result<_, Error> {
        let f = std::fs::File::open(zip).map_err(io(zip))?;
        zip::ZipArchive::new(std::io::BufReader::new(f)).map_err(bad)
    };
    let mut ar = open()?;
    let total: u64 = (0..ar.len())
        .filter_map(|i| ar.by_index_raw(i).ok().map(|f| f.size()))
        .sum();
    progress.start(Stage::Unzip, total);
    let n = ar.len();
    drop(ar);
    pool.install(|| {
        (0..n)
            .into_par_iter()
            .try_for_each_init(open, |ar, i| -> Result<(), Error> {
                let ar = ar
                    .as_mut()
                    .map_err(|e| io(zip)(std::io::Error::other(e.to_string())))?;
                progress.check()?;
                let mut f = ar.by_index(i).map_err(bad)?;
                let name = f
                    .enclosed_name()
                    .filter(|p| !f.is_dir() && !p.starts_with("__MACOSX"))
                    .and_then(|p| p.file_name().map(PathBuf::from));
                let Some(name) = name else { return Ok(()) };
                let out = dest.join(name);
                let mut w = std::fs::File::create(&out).map_err(io(&out))?;
                let size = std::io::copy(&mut f, &mut w).map_err(io(&out))?;
                progress.add(Stage::Unzip, size);
                Ok(())
            })
    })?;
    progress.finish(Stage::Unzip);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn extracts_flat_once_and_rejects_traversal() {
        let tmp = std::env::temp_dir().join(format!("wpp-unzip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let zip_path = tmp.join("Conversa.zip");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let opt = zip::write::SimpleFileOptions::default();
        for (name, body) in [
            ("Chat/_chat.txt", "[06/01/2026, 10:00:00] Ana: oi\r\n"),
            ("Chat/00000001-AUDIO-2026-01-06-10-00-00.opus", "x"),
            ("__MACOSX/Chat/._chat.txt", "junk"),
            ("../evil.txt", "no"),
        ] {
            w.start_file(name, opt).unwrap();
            w.write_all(body.as_bytes()).unwrap();
        }
        w.finish().unwrap();

        let pool = crate::new_pool().unwrap();
        let p = Progress::default();
        let dir = folder_for(&zip_path, &tmp.join("zips"), &pool, &p).unwrap();
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            ["00000001-AUDIO-2026-01-06-10-00-00.opus", "_chat.txt"]
        );
        assert!(!tmp.join("evil.txt").exists());
        assert!((p.fraction() - 0.7).abs() < 1e-9);
        assert_eq!(
            folder_for(&zip_path, &tmp.join("zips"), &pool, &p).unwrap(),
            dir
        );
        let a = crate::analyze(&dir, &pool).unwrap();
        assert_eq!(a.events.len(), 1);
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
