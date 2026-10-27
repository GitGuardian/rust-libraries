//! The `gitguardian-client` crate provides wrappers for HTTP clients
//! to interact with GitGuardian's public API.
//! It relies on the `gitguardian-api` crate for definitions of the API's models and routes.
//!
//! # Example
//!
//! Paginating through the teams of a workspace with a custom [`::ureq::Agent`]. See
//! [`ureq::Client::with_agent`] for how the agent is expected to be configured.
//!
//! ```no_run
//! # #[cfg(feature = "ureq")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use std::time::Duration;
//!
//! use gitguardian_client::api::ApiConfig;
//! use gitguardian_client::api::api::v1::team::ListTeams;
//! use gitguardian_client::ureq::Client;
//!
//! let agent = ::ureq::Agent::config_builder()
//!     .http_status_as_error(false)
//!     .timeout_global(Some(Duration::from_secs(30)))
//!     .build()
//!     .new_agent();
//!
//! let api_key = std::env::var("GITGUARDIAN_API_KEY")?;
//! let client = Client::with_agent(ApiConfig::new(&api_key)?, agent);
//!
//! for page in client.paginate(ListTeams::default()) {
//!     for team in page?.items {
//!         println!("{}", team.name);
//!     }
//! }
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "ureq"))]
//! # fn main() {}
//! ```
mod error;

#[cfg(feature = "reqwest")]
pub mod reqwest;
#[cfg(feature = "ureq")]
pub mod ureq;

pub use gitguardian_api as api;

pub use error::Error;
