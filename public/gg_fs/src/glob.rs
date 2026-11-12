//! Exclusion patterns.

use std::path::Path;

use regex::{Regex, RegexSet};

/// An error from parsing a pattern.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
#[error("invalid pattern {pattern:?} at byte {offset}: {message}")]
pub struct PatternError {
    /// The pattern that failed to parse.
    pub pattern: String,
    /// The byte offset of the error.
    pub offset: usize,
    /// A description of the error.
    pub message: &'static str,
}

/// A set of compiled patterns.
///
/// `*` matches within a path component and `**` across components. A path
/// matches the set if it matches any pattern.
#[derive(Debug, Clone, Default)]
pub struct Patterns {
    set: RegexSet,
}

impl Patterns {
    /// Compiles `patterns`.
    ///
    /// # Errors
    ///
    /// This function will return the [`PatternError`] of the first invalid
    /// pattern.
    pub fn new<S: AsRef<str>>(patterns: impl IntoIterator<Item = S>) -> Result<Self, PatternError> {
        let patterns: Vec<String> = patterns.into_iter().map(|p| p.as_ref().to_owned()).collect();
        let translated = patterns
            .iter()
            .map(|pattern| {
                translate(pattern).map_err(|(offset, message)| error(pattern, offset, message))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let set = RegexSet::new(&translated).map_err(|_| {
            let invalid = patterns.iter().zip(&translated).find(|(_, t)| Regex::new(t).is_err());
            error(invalid.map_or("", |(pattern, _)| pattern), 0, "too large to compile")
        })?;
        Ok(Self { set })
    }

    /// Returns `true` if any pattern matches `path`.
    ///
    /// Backslashes in `path` are converted to forward slashes before matching.
    /// The path is matched as given, so it should be relative to the project
    /// root.
    pub fn matches(&self, path: impl AsRef<Path>) -> bool {
        self.set.is_match(&path.as_ref().to_string_lossy().replace('\\', "/"))
    }
}

fn error(pattern: &str, offset: usize, message: &'static str) -> PatternError {
    PatternError { pattern: pattern.to_owned(), offset, message }
}

fn translate(pattern: &str) -> Result<String, (usize, &'static str)> {
    validate(pattern)?;
    let pattern = collapse_double_stars(pattern);
    if pattern == "**" {
        Ok("(?s:.+)".to_owned())
    } else {
        let (mut regex, mut rest) = match pattern.strip_prefix('/') {
            Some(rest) => ("^".to_owned(), rest),
            None => ("(?:^|/)".to_owned(), pattern.as_str()),
        };
        let mut open_ended = false;
        while let Some(c) = rest.chars().next() {
            if rest == "/**" {
                regex.push_str("(?s:/.*)?$");
                open_ended = true;
                break;
            } else if let Some(tail) = rest.strip_prefix("**/") {
                regex.push_str("(?:[^/]+/)*");
                rest = tail;
            } else if let Some(tail) = rest.strip_prefix('*') {
                regex.push_str("[^/]+");
                rest = tail;
            } else {
                let (head, tail) = rest.split_at(c.len_utf8());
                regex.push_str(&regex::escape(head));
                rest = tail;
            }
        }
        if !open_ended && !pattern.ends_with('/') {
            regex.push('$');
        }
        Ok(regex)
    }
}

fn validate(pattern: &str) -> Result<(), (usize, &'static str)> {
    if pattern.is_empty() {
        Err((0, "empty pattern"))
    } else if pattern != "**" && pattern.split('/').all(|part| part.is_empty() || part == "**") {
        Err((0, "matches every path; use `**` to match everything"))
    } else if let Some(offset) = pattern.find("//") {
        Err((offset, "empty path component"))
    } else if let Some(offset) = pattern.find("***") {
        Err((offset, "`***` is not allowed"))
    } else {
        let bytes = pattern.as_bytes();
        pattern.match_indices("**").try_for_each(|(offset, _)| {
            let before = offset.checked_sub(1).and_then(|i| bytes.get(i));
            let after = bytes.get(offset + 2);
            if before.is_some_and(|b| *b != b'/') {
                Err((offset, "`**` must be at the start or follow `/`"))
            } else if after.is_some_and(|b| *b != b'/') {
                Err((offset, "`**` must be at the end or be followed by `/`"))
            } else {
                Ok(())
            }
        })
    }
}

fn collapse_double_stars(pattern: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for segment in pattern.split('/') {
        if segment != "**" || segments.last() != Some(&"**") {
            segments.push(segment);
        }
    }
    segments.join("/")
}
