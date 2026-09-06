//! Token helpers on top of `weavatrix-parse`. Trivia is dropped.

use weavatrix_parse::{Language, Token, TokenKind, tokenize_lite};

/// Significant token stream for one file.
pub struct Stream<'source> {
    source: &'source str,
    tokens: Vec<Token>,
}

impl<'source> Stream<'source> {
    /// Tokenizes `source` in lite mode.
    #[must_use]
    pub fn new(source: &'source str, language: Language) -> Self {
        Self {
            source,
            tokens: tokenize_lite(source, language),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    #[must_use]
    pub fn kind(&self, index: usize) -> Option<TokenKind> {
        self.tokens.get(index).map(|token| token.kind)
    }

    #[must_use]
    pub fn text(&self, index: usize) -> &str {
        self.tokens
            .get(index)
            .map_or("", |token| token.text(self.source))
    }

    #[must_use]
    pub fn ident(&self, index: usize) -> Option<&str> {
        (self.kind(index) == Some(TokenKind::Identifier)).then(|| self.text(index))
    }

    #[must_use]
    pub fn is_punct(&self, index: usize, mark: &str) -> bool {
        self.kind(index) == Some(TokenKind::Punctuation) && self.text(index) == mark
    }

    /// Unquoted string literal at `index`.
    #[must_use]
    pub fn string(&self, index: usize) -> Option<String> {
        if self.kind(index) != Some(TokenKind::String) {
            return None;
        }
        Some(unquote(self.text(index)))
    }

    /// Local `NAME = "literal"`, `NAME := "literal"`, or
    /// `NAME = … || "literal"` / `NAME = … ?? "literal"` fallback bindings.
    #[must_use]
    pub fn bindings(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut index = 0;
        while index + 2 < self.len() {
            if let Some(name) = self.ident(index) {
                let operator = index + 1;
                let assigned = self.is_punct(operator, "=")
                    || (self.is_punct(operator, ":") && self.is_punct(operator + 1, "="));
                let value_index =
                    if self.is_punct(operator, ":") && self.is_punct(operator + 1, "=") {
                        operator + 2
                    } else {
                        operator + 1
                    };
                if assigned {
                    if let Some(literal) = self.string(value_index) {
                        out.push((name.to_owned(), literal));
                    } else if let Some(literal) = self.fallback_string(value_index) {
                        out.push((name.to_owned(), literal));
                    }
                }
            }
            index += 1;
        }
        out
    }

    /// Scans a short RHS for `|| "literal"` / `?? "literal"` (env defaults).
    fn fallback_string(&self, start: usize) -> Option<String> {
        let end = (start + 24).min(self.len());
        let mut cursor = start;
        while cursor + 1 < end {
            if self.is_punct(cursor, ";") || self.is_punct(cursor, ",") {
                break;
            }
            if (self.is_punct(cursor, "|") && self.is_punct(cursor + 1, "|"))
                || (self.is_punct(cursor, "?") && self.is_punct(cursor + 1, "?"))
            {
                let mut look = cursor + 2;
                while look < end {
                    if let Some(literal) = self.string(look) {
                        return Some(literal);
                    }
                    if self.is_punct(look, ";") || self.is_punct(look, ",") {
                        break;
                    }
                    look += 1;
                }
            }
            cursor += 1;
        }
        None
    }

    /// `key: "value"` / `key = "value"` property strings, including `topics = ["a"]`.
    #[must_use]
    pub fn properties(&self, keys: &[&str]) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut index = 0;
        while index + 2 < self.len() {
            if let Some(name) = self.ident(index)
                && keys.iter().any(|key| name.eq_ignore_ascii_case(key))
            {
                let mut cursor = index + 1;
                if self.is_punct(cursor, ":") || self.is_punct(cursor, "=") {
                    cursor += 1;
                }
                if self.is_punct(cursor, "[") {
                    cursor += 1;
                }
                if let Some(literal) = self.string(cursor) {
                    out.push((name.to_ascii_lowercase(), literal));
                } else if let Some(ident) = self.ident(cursor) {
                    out.push((name.to_ascii_lowercase(), ident.to_owned()));
                }
            }
            index += 1;
        }
        out
    }
}

/// Strips quotes and simple escapes. Raw credentials must not be logged after this.
#[must_use]
pub fn unquote(raw: &str) -> String {
    let trimmed = raw.trim();
    let Some(start) = trimmed.find(['"', '\'', '`']) else {
        return trimmed.to_owned();
    };
    let bytes = trimmed.as_bytes();
    if start >= bytes.len() {
        return trimmed.to_owned();
    }
    let quote = bytes[start];
    // `start` is an ASCII quote byte, so `start + 1` is a char boundary.
    let rest = &trimmed[start + 1..];
    let Some(end) = rest.as_bytes().iter().rposition(|&b| b == quote) else {
        return rest.to_owned();
    };
    // `end` indexes an ASCII quote in `rest`, so it is a char boundary.
    unescape(&rest[..end])
}

fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character == '\\'
            && let Some(next) = chars.next()
        {
            match next {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                other => out.push(other),
            }
        } else {
            out.push(character);
        }
    }
    out
}
