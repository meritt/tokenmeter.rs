use std::sync::LazyLock;

use rustc_hash::FxHashMap;
use tiktoken_rs::{CoreBPE, O200K_BASE_PAT_STR, Rank};
use unicode_segmentation::UnicodeSegmentation;

pub const ENCODING: &str = "o200k_base";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    pub end: usize,
    pub tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Split {
    pub tokens: usize,
    pub groups: Vec<Group>,
    pub truncated: bool,
}

static TABLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/o200k_base.bin"));

struct Encoding {
    core: CoreBPE,
    tokens: Vec<&'static [u8]>,
}

static ENCODING_TABLES: LazyLock<Encoding> = LazyLock::new(|| {
    let mut tokens = Vec::with_capacity(200_000);
    let mut at = 0;

    while at < TABLE.len() {
        let end = at + 1 + usize::from(TABLE[at]);

        tokens.push(&TABLE[at + 1..end]);
        at = end;
    }

    let encoder: FxHashMap<Vec<u8>, Rank> =
        tokens.iter().enumerate().map(|(rank, token)| (token.to_vec(), rank as Rank)).collect();
    let core = CoreBPE::new(encoder, FxHashMap::default(), O200K_BASE_PAT_STR).expect("the o200k_base pattern");

    Encoding { core, tokens }
});

pub fn prepare() {
    LazyLock::force(&ENCODING_TABLES);
}

#[must_use]
pub fn count(text: &str) -> usize {
    encode(text).len()
}

#[must_use]
pub fn encode(text: &str) -> Vec<u32> {
    ENCODING_TABLES.core.encode_ordinary(text)
}

#[must_use]
pub fn split(text: &str, limit: usize) -> Split {
    let ids = encode(text);
    let mut graphemes = text.grapheme_indices(true).map(|(start, grapheme)| start + grapheme.len());
    let mut boundary = 0;
    let mut groups = Vec::new();
    let mut offset = 0;
    let mut pending = 0;

    for &id in &ids {
        offset += ENCODING_TABLES.tokens[id as usize].len();
        pending += 1;

        while boundary < offset {
            boundary = graphemes.next().unwrap_or(text.len());
        }

        if boundary != offset {
            continue;
        }

        if groups.len() == limit {
            return Split { tokens: ids.len(), groups, truncated: true };
        }

        groups.push(Group { end: offset, tokens: pending });
        pending = 0;
    }

    Split { tokens: ids.len(), groups, truncated: false }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(text: &str, split: &Split) -> Vec<String> {
        let mut start = 0;

        split
            .groups
            .iter()
            .map(|group| {
                let piece = text[start..group.end].to_owned();
                start = group.end;
                piece
            })
            .collect()
    }

    #[test]
    fn counts_a_phrase_from_the_site() {
        assert_eq!(count("Пользователи — это наше всё."), 9);
    }

    #[test]
    fn counts_special_tokens_as_text() {
        assert!(count("<|endoftext|>") > 1);
    }

    #[test]
    fn counts_nothing_in_empty_text() {
        assert_eq!(count(""), 0);
        assert_eq!(split("", 200), Split { tokens: 0, groups: vec![], truncated: false });
    }

    #[test]
    fn groups_cover_the_text() {
        let text = "Съешь же ещё этих мягких французских булок, да выпей чаю.\n\n  fn main() {}";
        let split = split(text, 200);

        assert_eq!(texts(text, &split).concat(), text);
        assert_eq!(split.groups.iter().map(|group| group.tokens).sum::<usize>(), split.tokens);
        assert_eq!(split.tokens, count(text));
        assert!(!split.truncated);
    }

    #[test]
    fn keeps_graphemes_whole() {
        let text = "👩🏽‍💻 é 🇷🇺 ไทย";
        let split = split(text, 200);

        for piece in texts(text, &split) {
            assert_eq!(piece.graphemes(true).collect::<String>(), piece);
        }

        assert!(texts(text, &split).iter().any(|piece| piece.contains("👩🏽‍💻")));
        assert_eq!(split.groups.last().map(|group| group.end), Some(text.len()));
    }

    #[test]
    fn stops_at_the_limit_and_counts_the_rest() {
        let text = "слово ".repeat(1000);
        let split = split(&text, 200);

        assert_eq!(split.groups.len(), 200);
        assert!(split.truncated);
        assert_eq!(split.tokens, count(&text));
    }
}
