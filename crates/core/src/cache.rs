use crate::media::Fingerprint;
use crate::Analysis;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const SCHEMA: u32 = 3;

pub fn default_dir() -> Option<PathBuf> {
    crate::app_dir().map(|d| d.join("cache"))
}

fn key(folder: &Path) -> String {
    let p = folder
        .canonicalize()
        .unwrap_or_else(|_| folder.to_path_buf());
    p.to_string_lossy().into_owned()
}

fn file(cache: &Path, key: &str) -> PathBuf {
    let h = Sha256::digest(key.as_bytes());
    cache.join(crate::hex(&h[..16]) + ".bin")
}

pub fn load(cache: &Path, folder: &Path, fp: Fingerprint) -> Option<Analysis> {
    let key = key(folder);
    let path = file(cache, &key);
    let bytes = std::fs::read(&path).ok()?;
    let (schema, rest) = bytes.split_at_checked(4)?;
    if schema != SCHEMA.to_le_bytes() {
        let _ = std::fs::remove_file(&path);
        return None;
    }
    let raw = zstd::decode_all(rest).ok()?;
    let (k, f, a): (String, Fingerprint, Analysis) = bincode::deserialize(&raw).ok()?;
    (k == key && f == fp).then_some(a)
}

pub fn store(cache: &Path, folder: &Path, fp: Fingerprint, a: &Analysis) -> std::io::Result<()> {
    let key = key(folder);
    let raw = bincode::serialize(&(&key, fp, a)).map_err(std::io::Error::other)?;
    let mut out = SCHEMA.to_le_bytes().to_vec();
    out.extend(zstd::encode_all(&raw[..], 1)?);
    std::fs::create_dir_all(cache)?;
    let path = file(cache, &key);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, out)?;
    std::fs::rename(tmp, path)
}

pub fn clear(cache: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(cache) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}
