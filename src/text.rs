//! Accent-folded, word-bounded text matching shared by places, actors and equipment.
//! Tokens split on anything that is not a letter or digit, so "Mbuji-Mayi", "d'Uvira" and
//! "#M23" tokenize the same way in queries and in documents.
use std::collections::HashMap;

/// Lowercase and strip common Latin diacritics, one char in, one char out.
pub fn fold_char(c: char) -> char {
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        _ => c,
    }
}
pub fn fold(s: &str) -> String {
    s.chars().map(fold_char).collect()
}
pub fn tokens(s: &str) -> Vec<String> {
    fold(s)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect()
}

/// A search term; acronyms ("M23", "RDF", "CH-4") only match when written in capitals.
#[derive(Clone)]
pub struct Term {
    pub tokens: Vec<String>,
    pub acronym: bool,
}
pub fn term(raw: &str) -> Term {
    let raw = raw.trim();
    Term {
        tokens: tokens(raw),
        acronym: raw.chars().count() <= 6
            && raw.chars().any(char::is_alphabetic)
            && raw.chars().all(|c| !c.is_lowercase()),
    }
}

pub struct Token {
    pub start: usize,
    pub end: usize,
    pub folded: String,
    pub capitalized: bool,
}
pub struct Doc {
    pub chars: Vec<char>,
    pub tokens: Vec<Token>,
    index: HashMap<String, Vec<usize>>,
}
impl Doc {
    pub fn new(text: &str) -> Doc {
        let chars: Vec<char> = text.chars().collect();
        let mut tokens = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].is_alphanumeric() {
                let start = i;
                while i < chars.len() && chars[i].is_alphanumeric() {
                    i += 1;
                }
                tokens.push(Token {
                    start,
                    end: i,
                    folded: chars[start..i].iter().map(|&c| fold_char(c)).collect(),
                    capitalized: chars[start].is_uppercase(),
                });
            } else {
                i += 1;
            }
        }
        let mut index: HashMap<String, Vec<usize>> = HashMap::new();
        for (n, t) in tokens.iter().enumerate() {
            index.entry(t.folded.clone()).or_default().push(n);
        }
        Doc {
            chars,
            tokens,
            index,
        }
    }
    /// Token positions where the token sequence `needle` starts.
    pub fn find(&self, needle: &[String]) -> Vec<usize> {
        let Some(first) = needle.first() else {
            return Vec::new();
        };
        self.index
            .get(first)
            .into_iter()
            .flatten()
            .copied()
            .filter(|&i| {
                needle
                    .iter()
                    .enumerate()
                    .all(|(k, t)| self.tokens.get(i + k).is_some_and(|x| &x.folded == t))
            })
            .collect()
    }
    /// Token positions of a term; acronyms must appear in capitals.
    pub fn find_term(&self, t: &Term) -> Vec<usize> {
        self.find(&t.tokens)
            .into_iter()
            .filter(|&i| {
                !t.acronym
                    || self.chars[self.tokens[i].start..self.tokens[i].end]
                        .iter()
                        .all(|c| !c.is_lowercase())
            })
            .collect()
    }
    pub fn has_term(&self, t: &Term) -> bool {
        !self.find_term(t).is_empty()
    }
    pub fn has(&self, needle: &[String]) -> bool {
        !self.find(needle).is_empty()
    }
    /// Text between two tokens (exclusive).
    pub fn gap(&self, a: usize, b: usize) -> String {
        self.chars[self.tokens[a].end..self.tokens[b].start]
            .iter()
            .collect()
    }
    /// Char span of the sentence containing token `i`.
    pub fn sentence(&self, i: usize) -> (usize, usize) {
        let pos = self.tokens[i].start;
        let boundary = |c: char| matches!(c, '.' | '!' | '?' | '\n');
        let start = self.chars[..pos]
            .iter()
            .rposition(|&c| boundary(c))
            .map_or(0, |p| p + 1);
        let end = self.chars[pos..]
            .iter()
            .position(|&c| boundary(c))
            .map_or(self.chars.len(), |p| pos + p + 1);
        (start, end)
    }
    /// Tokens within a char span, for co-occurrence checks.
    pub fn span_doc(&self, (start, end): (usize, usize)) -> Doc {
        Doc::new(&self.chars[start..end].iter().collect::<String>())
    }
    /// About `radius` chars either side of the match, trimmed to word boundaries.
    pub fn snippet(&self, i: usize, len: usize, radius: usize) -> String {
        let a = self.tokens[i].start;
        let b = self.tokens[(i + len).saturating_sub(1).min(self.tokens.len() - 1)].end;
        let mut s = a.saturating_sub(radius);
        let mut e = (b + radius).min(self.chars.len());
        while s > 0 && !self.chars[s - 1].is_whitespace() {
            s += 1;
            if s >= a {
                break;
            }
        }
        while e < self.chars.len() && e > b && !self.chars[e].is_whitespace() {
            e -= 1;
        }
        let body: String = self.chars[s..e].iter().collect::<String>();
        let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
        format!(
            "{}{}{}",
            if s > 0 { "…" } else { "" },
            body,
            if e < self.chars.len() { "…" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folded_token_matching_respects_word_boundaries() {
        let d = Doc::new("Les combats à Mbuji-Mayi et à Bénin; l'#M23 avance. Bénéfices.");
        assert!(d.has(&tokens("mbuji mayi")));
        assert!(d.has(&tokens("M23")));
        assert!(d.has(&tokens("benin")));
        assert!(!d.has(&tokens("bene")));
        let i = d.find(&tokens("M23"))[0];
        let (s, e) = d.sentence(i);
        assert_eq!(
            d.chars[s..e].iter().collect::<String>(),
            "Les combats à Mbuji-Mayi et à Bénin; l'#M23 avance."
        );
        assert!(d.snippet(i, 1, 10).contains("M23"));
    }
}
