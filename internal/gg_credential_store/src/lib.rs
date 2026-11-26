#![doc = include_str!("../README.md")]

mod auth_config;
mod credential;
mod credential_store;
mod delete_outcome;
mod error;
mod list_outcome;
mod location;
mod machine_credential;
mod os_store;
mod os_store_error_reason;
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
pub use delete_outcome::DeleteOutcome;
pub use error::{Cause, CredentialStoreError};
pub use list_outcome::{ListOutcome, UnreadableEntry};
pub use location::Location;
pub use os_store_error_reason::OsStoreErrorReason;
pub use read_request::ReadRequest;
pub use setting::Setting;
pub use skip_reason::SkipReason;
pub use source::Source;
pub use store_outcome::StoreOutcome;
pub use stored_token::StoredToken;
pub use token_info::{TokenInfo, TokenType};
