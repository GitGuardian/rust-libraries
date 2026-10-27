//! URL handling for GitGuardian products

pub mod error;
pub mod generic_uri;
pub mod gitguardian;
pub mod host;
pub mod http_url;
pub mod placeholder;
pub mod query;
pub mod request_url;

pub use error::Error;
pub use generic_uri::GenericUri;
pub use gitguardian::{ApiUrl, DashboardUrl};
pub use host::Host;
pub use http_url::HttpUrl;
pub use placeholder::Template;
pub use query::Query;
pub use request_url::RequestUrl;
pub use url::Origin;
