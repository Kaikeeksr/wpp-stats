use crate::Kind;
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

pub const CHAT_FILE: &str = "_chat.txt";

const ANDROID_CHAT: &[&str] = &["WhatsApp Chat", "Conversa do WhatsApp"];

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub files: u64,
    pub bytes: u64,
    pub max_mtime_ns: u64,
}

#[derive(Debug, Default)]
pub struct MediaIndex {
    pub chat: Option<String>,
    pub entries: Vec<Entry>,
    pub fingerprint: Fingerprint,
    map: AHashMap<String, u32>,
}

fn chat_file(entries: &[Entry]) -> Option<usize> {
    if let Some(i) = entries
        .iter()
        .position(|e| e.name.eq_ignore_ascii_case(CHAT_FILE))
    {
        return Some(i);
    }
    let txt: Vec<usize> = (0..entries.len())
        .filter(|&i| entries[i].name.to_ascii_lowercase().ends_with(".txt"))
        .collect();
    match txt[..] {
        [i] => Some(i),
        _ => txt
            .into_iter()
            .find(|&i| ANDROID_CHAT.iter().any(|p| entries[i].name.starts_with(p))),
    }
}

impl MediaIndex {
    pub fn list(dir: &Path) -> std::io::Result<Self> {
        let mut entries = Vec::with_capacity(256);
        let mut fp = Fingerprint::default();
        for de in std::fs::read_dir(dir)? {
            let de = de?;
            let md = de.metadata()?;
            if !md.is_file() {
                continue;
            }
            fp.files += 1;
            fp.bytes += md.len();
            fp.max_mtime_ns = fp.max_mtime_ns.max(crate::mtime_ns(&md));
            entries.push(Entry {
                name: de.file_name().to_string_lossy().into_owned(),
                size: md.len(),
            });
        }
        let chat = chat_file(&entries).map(|i| entries.remove(i).name);
        let mut ix = Self::from_entries(entries);
        ix.chat = chat;
        ix.fingerprint = fp;
        Ok(ix)
    }

    pub fn from_entries(entries: Vec<Entry>) -> Self {
        let mut map = AHashMap::with_capacity(entries.len());
        for (i, e) in entries.iter().enumerate() {
            map.insert(name_key(&e.name), i as u32);
            if !e.name.is_ascii() {
                for table in [CP850, CP437] {
                    if let Some(fixed) = unmojibake(&e.name, table) {
                        map.entry(name_key(&fixed)).or_insert(i as u32);
                    }
                }
            }
        }
        Self {
            entries,
            map,
            ..Default::default()
        }
    }

    pub fn get(&self, name: &str) -> Option<u32> {
        self.map.get(&name_key(name)).copied()
    }
}

pub fn name_key(name: &str) -> String {
    if name.is_ascii() {
        name.to_ascii_lowercase()
    } else {
        name.nfc().collect::<String>().to_lowercase()
    }
}

const CP850: &str = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜø£Ø×ƒáíóúñÑªº¿®¬½¼¡«»░▒▓│┤ÁÂÀ©╣║╗╝¢¥┐\
                     └┴┬├─┼ãÃ╚╔╩╦╠═╬¤ðÐÊËÈıÍÎÏ┘┌█▄¦Ì▀ÓßÔÒõÕµþÞÚÛÙýÝ¯´\u{AD}±‗¾¶§÷¸°¨·¹³²■\u{A0}";
const CP437: &str = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐\
                     └┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■\u{A0}";

fn unmojibake(name: &str, table: &str) -> Option<String> {
    let bytes: Option<Vec<u8>> = name
        .chars()
        .map(|c| match u8::try_from(c) {
            Ok(b) if b.is_ascii() => Some(b),
            _ => table.chars().position(|t| t == c).map(|p| 0x80 + p as u8),
        })
        .collect();
    String::from_utf8(bytes?).ok().filter(|s| s != name)
}

pub fn kind_of(name: &str) -> Kind {
    for (marker, kind) in [
        ("-AUDIO-", Kind::Audio),
        ("-PHOTO-", Kind::Foto),
        ("-STICKER-", Kind::Figurinha),
    ] {
        if name.contains(marker) {
            return kind;
        }
    }
    let ext = name.rsplit_once('.').map_or("", |(_, e)| e);
    match ext.to_ascii_lowercase().as_str() {
        "opus" | "m4a" | "mp3" => Kind::Audio,
        "jpg" | "jpeg" | "png" | "heic" => Kind::Foto,
        "webp" => Kind::Figurinha,
        "mp4" | "mov" => Kind::Video,
        "gif" => Kind::Gif,
        _ => Kind::Documento,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfc_and_nfd_names_match() {
        let nfd = "00000531-curri\u{301}culo.pdf";
        let nfc = "00000531-currículo.pdf";
        assert_ne!(nfd, nfc);
        let idx = MediaIndex::from_entries(vec![Entry {
            name: nfd.into(),
            size: 7,
        }]);
        assert_eq!(idx.get(nfc), Some(0));
        assert_eq!(idx.get("00000531-CURRÍCULO.PDF"), Some(0));
        assert_eq!(idx.get("outro.pdf"), None);
    }

    #[test]
    fn code_page_mojibake_on_disk_matches() {
        assert_eq!(CP850.chars().count(), 128);
        assert_eq!(CP437.chars().count(), 128);
        let disk = "00000703-GSS ÔÇö Gesta╠âo.mp4";
        let idx = MediaIndex::from_entries(vec![Entry {
            name: disk.into(),
            size: 1,
        }]);
        assert_eq!(idx.get("00000703-GSS — Gesta\u{303}o.mp4"), Some(0));
        assert_eq!(idx.get("00000703-GSS — Gestão.mp4"), Some(0));
        assert_eq!(idx.get(disk), Some(0));
        assert_eq!(unmojibake("café.jpg", CP850), None);
    }

    #[test]
    fn kinds() {
        assert_eq!(
            kind_of("00000032-AUDIO-2026-01-06-09-45-38.opus"),
            Kind::Audio
        );
        assert_eq!(
            kind_of("00000001-PHOTO-2026-01-06-09-45-38.jpg"),
            Kind::Foto
        );
        assert_eq!(
            kind_of("00000027-STICKER-2026-01-06-09-45-04.webp"),
            Kind::Figurinha
        );
        assert_eq!(kind_of("00000703-gravação.MP4"), Kind::Video);
        assert_eq!(kind_of("00000531-cv.pdf"), Kind::Documento);
        assert_eq!(kind_of("x.gif"), Kind::Gif);
        assert_eq!(kind_of("sem_extensao"), Kind::Documento);
        assert_eq!(kind_of("PTT-20250325-WA0001.opus"), Kind::Audio);
        assert_eq!(kind_of("STK-20250325-WA0002.webp"), Kind::Figurinha);
        assert_eq!(kind_of("Contato.vcf"), Kind::Documento);
    }

    #[test]
    fn finds_the_chat_file() {
        let e = |n: &str| Entry {
            name: n.into(),
            size: 1,
        };
        let pick = |v: Vec<Entry>| chat_file(&v).map(|i| v[i].name.clone());
        assert_eq!(
            pick(vec![e("a.jpg"), e("_chat.txt")]).as_deref(),
            Some("_chat.txt")
        );
        assert_eq!(
            pick(vec![e("a.jpg"), e("WhatsApp Chat with Ana.txt")]).as_deref(),
            Some("WhatsApp Chat with Ana.txt")
        );
        assert_eq!(
            pick(vec![e("notas.txt"), e("Conversa do WhatsApp com Ana.txt")]).as_deref(),
            Some("Conversa do WhatsApp com Ana.txt")
        );
        assert_eq!(pick(vec![e("a.txt"), e("b.txt")]), None);
        assert_eq!(pick(vec![e("a.jpg")]), None);
    }
}
