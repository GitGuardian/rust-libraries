use url::Url;

use crate::models::pagination::Cursor;

#[derive(Clone, Copy, Debug)]
/// Pagination cursor directions
pub enum Rel {
    Next,
    Prev,
}

impl Rel {
    /// Cursor suffixes used by the GitGuardian API
    fn suffix(self) -> &'static str {
        match self {
            Self::Next => ">; rel=\"next\"",
            Self::Prev => ">; rel=\"prev\"",
        }
    }
}

/// Retrieves a pagination cursor from a link header value
pub fn cursor(link: &str, rel: Rel) -> Option<Cursor> {
    link.split(',').find_map(|entry| {
        let target = entry.trim().strip_prefix('<')?.strip_suffix(rel.suffix())?;
        Url::parse(target)
            .ok()?
            .query_pairs()
            .find(|(name, _)| name == "cursor")
            .map(|(_, value)| Cursor::from(value.into_owned()))
    })
}
