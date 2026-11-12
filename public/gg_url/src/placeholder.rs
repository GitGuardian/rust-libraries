//! `{name}` placeholders in URL templates.

use std::borrow::Cow;
use std::sync::LazyLock;

use regex::{NoExpand, Regex};

static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{(?:(?P<name>[\w.]+)|[^}]*)\}").expect("placeholder regex is valid")
});

/// Text that may hold `{name}` placeholders, such as a URL template.
pub trait Template {
    /// The `{name}` placeholders, in order.
    fn placeholders(&self) -> impl Iterator<Item = &str>;

    /// Whether at least one `{name}` placeholder is left.
    fn has_placeholder(&self) -> bool {
        self.placeholders().next().is_some()
    }

    /// This text with every `{...}` replaced by `with`.
    fn fill_placeholders(&self, with: &str) -> Cow<'_, str>;
}

impl Template for str {
    fn placeholders(&self) -> impl Iterator<Item = &str> {
        PLACEHOLDER
            .captures_iter(self)
            .filter(|caps| caps.name("name").is_some())
            .map(|caps| caps.get(0).expect("group 0 is the whole match").as_str())
    }

    fn fill_placeholders(&self, with: &str) -> Cow<'_, str> {
        PLACEHOLDER.replace_all(self, NoExpand(with))
    }
}

#[cfg(test)]
mod tests {
    use yare::parameterized;

    use super::*;

    /// GIVEN a string with braces
    /// WHEN listing its placeholders
    /// THEN only braces holding a bare name count, in order
    #[parameterized(
        url = { "https://{region}.example.com/{org}", &["{region}", "{org}"] },
        dotted = { "{extra.token}", &["{extra.token}"] },
        repeated = { "{a}/{a}", &["{a}", "{a}"] },
        empty = { "?param={}", &[] },
        not_a_name = { "{c d}/{env:X}/{ss@h}", &[] },
        graphql = { "{actor{user{name}}}", &[] },
        lone_brace = { "https://u:p{ss@example.com", &[] },
    )]
    fn lists_placeholders(input: &str, expected: &[&str]) {
        assert_eq!(input.placeholders().collect::<Vec<_>>(), expected);
    }

    #[parameterized(
        url = { "https://{region}.example.com/{org}", "x", "https://x.example.com/x" },
        any_braces = { "?param={}&q={c d}", "x", "?param=x&q=x" },
        dollar_index = { "{token}", "abc$1", "abc$1" },
        dollar_name = { "{token}", "p$ss${name}x", "p$ss${name}x" },
        dollar_zero = { "a/{token}/b", "$0", "a/$0/b" },
    )]
    fn fills_placeholders_literally(input: &str, with: &str, expected: &str) {
        assert_eq!(input.fill_placeholders(with), expected);
    }
}
