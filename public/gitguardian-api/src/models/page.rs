use crate::models::pagination::Cursor;

#[derive(Clone, Debug)]
/// Generic page from a paginated result
pub struct Page<T> {
    pub items: Vec<T>,
    pub previous: Option<Cursor>,
    pub next: Option<Cursor>,
}
