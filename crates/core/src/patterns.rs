use aho_corasick::{AhoCorasick, Anchored, Input, MatchKind, StartKind};
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pat {
    System,
    Deleted,
    FotoUnica,
    VideoUnico,
    AudioUnico,
    Other,
}

const ANCHORED: &[(Pat, bool, &str)] = &[
    (
        Pat::System,
        false,
        "As mensagens e ligações são protegidas com a criptografia de ponta a ponta",
    ),
    (Pat::Deleted, true, "Mensagem apagada"),
    (Pat::FotoUnica, true, "imagem ocultada"),
    (Pat::VideoUnico, true, "vídeo omitido"),
    (Pat::AudioUnico, true, "áudio ocultado"),
    (Pat::AudioUnico, true, "áudio omitido"),
    (
        Pat::System,
        false,
        "Messages and calls are end-to-end encrypted",
    ),
    (Pat::Deleted, true, "This message was deleted"),
    (Pat::FotoUnica, true, "image omitted"),
    (Pat::VideoUnico, true, "video omitted"),
    (Pat::AudioUnico, true, "audio omitted"),
    (Pat::Deleted, true, "You deleted this message"),
    (Pat::AudioUnico, true, "<View once voice message omitted>"),
    (Pat::Other, true, "<Media omitted>"),
    (Pat::Other, false, "location: "),
    (Pat::Deleted, true, "Você apagou esta mensagem"),
    (Pat::Other, true, "<Mídia oculta>"),
];

const ATTACH: &[&str] = &["<anexado: ", "<attached: "];

const ATTACHED: &[&str] = &[" (file attached)", " (arquivo anexado)"];

const EDITED: &[&str] = &["<Mensagem editada>", "<This message was edited>"];

const LINKS: &[&str] = &["https://", "http://", "www."];
const WWW: usize = 2;

const PREFIX_LEN: usize = 128;

pub(crate) const LRM: &[u8] = "\u{200E}".as_bytes();

pub(crate) struct Patterns {
    anchored: AhoCorasick,
    attach: AhoCorasick,
    attached: AhoCorasick,
    links: AhoCorasick,
}

pub(crate) static PATTERNS: LazyLock<Patterns> = LazyLock::new(|| Patterns {
    anchored: AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .start_kind(StartKind::Anchored)
        .build(ANCHORED.iter().map(|p| p.2))
        .expect("static patterns"),
    attach: AhoCorasick::new(ATTACH).expect("static patterns"),
    attached: AhoCorasick::new(ATTACHED).expect("static patterns"),
    links: AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostFirst)
        .ascii_case_insensitive(true)
        .build(LINKS)
        .expect("static patterns"),
});

impl Patterns {
    pub fn classify(&self, body: &[u8]) -> Option<Pat> {
        let body = body.trim_ascii_end();
        let head = &body[..body.len().min(PREFIX_LEN)];
        let m = self
            .anchored
            .find(Input::new(head).anchored(Anchored::Yes))?;
        let (pat, exact, _) = ANCHORED[m.pattern().as_usize()];
        (!exact || m.end() == body.len()).then_some(pat)
    }

    pub fn find_attach(&self, body: &[u8]) -> Option<(usize, usize)> {
        self.attach.find(body).map(|m| (m.start(), m.end()))
    }

    pub fn find_attached(&self, body: &[u8]) -> Option<(usize, usize)> {
        let line = &body[..memchr::memchr(b'\n', body).unwrap_or(body.len())];
        let m = self.attached.find(line)?;
        (m.end() == line.trim_ascii_end().len())
            .then(|| (m.start(), (line.len() + 1).min(body.len())))
    }

    pub fn count_links(&self, text: &[u8]) -> u32 {
        self.links
            .find_iter(text)
            .filter(|m| {
                m.pattern().as_usize() != WWW
                    || m.start() == 0
                    || !(text[m.start() - 1] == b'/' || text[m.start() - 1].is_ascii_alphanumeric())
            })
            .count() as u32
    }

    pub fn strip_edited<'a>(&self, body: &'a [u8]) -> Option<&'a [u8]> {
        let b = EDITED
            .iter()
            .find_map(|s| body.strip_suffix(s.as_bytes()))?;
        let b = b.strip_suffix(LRM).unwrap_or(b);
        Some(b.strip_suffix(b" ").unwrap_or(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links() {
        let n = |t: &str| PATTERNS.count_links(t.as_bytes());
        assert_eq!(n("olha https://www.exemplo.com e http://x.y"), 2);
        assert_eq!(n("www.site.com.br, WWW.OUTRO.COM"), 2);
        assert_eq!(n("awww... que fofo"), 0);
        assert_eq!(n("sem link"), 0);
    }
}
