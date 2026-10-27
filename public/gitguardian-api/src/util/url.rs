use serde::Serialize;
use url::Url;

use crate::error::BuildError;

pub fn set_query<T: Serialize>(url: &mut Url, query: &T) -> Result<(), BuildError> {
    let encoded = serde_urlencoded::to_string(query)?;
    url.set_query((!encoded.is_empty()).then_some(&encoded));
    Ok(())
}
