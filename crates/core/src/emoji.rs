use std::iter::Peekable;
use std::str::CharIndices;

const VS16: char = '\u{FE0F}';
const ZWJ: char = '\u{200D}';
const KEYCAP: char = '\u{20E3}';

const PRESENTATION: &[(u32, u32)] = &[
    (0x231A, 0x231B),
    (0x23E9, 0x23EC),
    (0x23F0, 0x23F0),
    (0x23F3, 0x23F3),
    (0x25FD, 0x25FE),
    (0x2614, 0x2615),
    (0x2648, 0x2653),
    (0x267F, 0x267F),
    (0x2693, 0x2693),
    (0x26A1, 0x26A1),
    (0x26AA, 0x26AB),
    (0x26BD, 0x26BE),
    (0x26C4, 0x26C5),
    (0x26CE, 0x26CE),
    (0x26D4, 0x26D4),
    (0x26EA, 0x26EA),
    (0x26F2, 0x26F3),
    (0x26F5, 0x26F5),
    (0x26FA, 0x26FA),
    (0x26FD, 0x26FD),
    (0x2705, 0x2705),
    (0x270A, 0x270B),
    (0x2728, 0x2728),
    (0x274C, 0x274C),
    (0x274E, 0x274E),
    (0x2753, 0x2755),
    (0x2757, 0x2757),
    (0x2795, 0x2797),
    (0x27B0, 0x27B0),
    (0x27BF, 0x27BF),
    (0x2B1B, 0x2B1C),
    (0x2B50, 0x2B50),
    (0x2B55, 0x2B55),
    (0x1F004, 0x1F004),
    (0x1F0CF, 0x1F0CF),
    (0x1F18E, 0x1F18E),
    (0x1F191, 0x1F19A),
    (0x1F201, 0x1F201),
    (0x1F21A, 0x1F21A),
    (0x1F22F, 0x1F22F),
    (0x1F232, 0x1F236),
    (0x1F238, 0x1F23A),
    (0x1F250, 0x1F251),
    (0x1F300, 0x1F64F),
    (0x1F680, 0x1F6FF),
    (0x1F7E0, 0x1F7FF),
    (0x1F900, 0x1F9FF),
    (0x1FA70, 0x1FAFF),
];

fn presentation(c: char) -> bool {
    let c = c as u32;
    PRESENTATION
        .binary_search_by(|&(lo, hi)| {
            if hi < c {
                std::cmp::Ordering::Less
            } else if lo > c {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

fn regional(c: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&c)
}

fn modifier(c: char) -> bool {
    matches!(c, VS16 | KEYCAP | '\u{1F3FB}'..='\u{1F3FF}' | '\u{E0020}'..='\u{E007F}')
}

fn peek(it: &mut Peekable<CharIndices>) -> Option<char> {
    it.peek().map(|&(_, c)| c)
}

pub(crate) fn scan(s: &str, mut f: impl FnMut(&str)) {
    let mut it = s.char_indices().peekable();
    while let Some((start, c)) = it.next() {
        let found = if regional(c) {
            peek(&mut it).is_some_and(regional) && it.next().is_some()
        } else if c.is_ascii_digit() || c == '#' || c == '*' {
            let mut j = it.clone();
            if peek(&mut j) == Some(VS16) {
                j.next();
            }
            let keycap = peek(&mut j) == Some(KEYCAP);
            if keycap {
                it = j;
            }
            keycap
        } else {
            (c >= '\u{231A}' && presentation(c)) || (!c.is_ascii() && peek(&mut it) == Some(VS16))
        };
        if !found {
            continue;
        }
        loop {
            match peek(&mut it) {
                Some(m) if modifier(m) => {
                    it.next();
                }
                Some(ZWJ) => {
                    let mut j = it.clone();
                    j.next();
                    match peek(&mut j) {
                        Some(n) if !n.is_ascii() && n != ZWJ => {
                            j.next();
                            it = j;
                        }
                        _ => break,
                    }
                }
                _ => break,
            }
        }
        let end = it.peek().map_or(s.len(), |&(i, _)| i);
        f(&s[start..end]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(s: &str) -> Vec<String> {
        let mut v = Vec::new();
        scan(s, |e| v.push(e.to_string()));
        v
    }

    #[test]
    fn clusters() {
        assert_eq!(all("oi 😂😂 kkk"), ["😂", "😂"]);
        assert_eq!(all("te amo ❤️ ❤ fim"), ["❤️"]);
        assert_eq!(all("👍🏽👨‍👩‍👧🤷‍♀️"), ["👍🏽", "👨‍👩‍👧", "🤷‍♀️"]);
        assert_eq!(all("🇧🇷🇵🇹 🏳️‍🌈"), ["🇧🇷", "🇵🇹", "🏳️‍🌈"]);
        assert_eq!(all("1️⃣ #⃣ 2 # * 10"), ["1️⃣", "#⃣"]);
        assert_eq!(
            all("🏴\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}").len(),
            1
        );
        assert_eq!(all("⚽✅⭐"), ["⚽", "✅", "⭐"]);
        assert!(all("café © ™ — “aspas” ação ½ \u{200E}").is_empty());
        assert!(all("\u{1F1E7}").is_empty());
        assert_eq!(all("😀\u{200D}"), ["😀"]);
    }
}
