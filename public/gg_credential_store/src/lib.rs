#![doc = include_str!("../README.md")]

mod auth_config;
mod credential;
mod credential_store;
mod error;
mod location;
mod os_store;
mod read_request;
mod setting;
mod skip_reason;
mod source;
mod store_outcome;
mod stored_token;
mod token_info;

pub use gg_url;

pub use credential::Credential;
pub use credential_store::{CredentialStore, MACHINE_TOKEN_PREFIX};
pub use error::{Cause, CredentialStoreError};
pub use location::Location;
#[cfg(feature = "in-memory-store")]
pub use os_store::{InMemoryOsStore, OsStoreFailure};
pub use read_request::ReadRequest;
pub use setting::Setting;
pub use skip_reason::SkipReason;
pub use source::Source;
pub use store_outcome::{CleartextReason, StoreOutcome};
pub use stored_token::StoredToken;
pub use token_info::TokenInfo;
