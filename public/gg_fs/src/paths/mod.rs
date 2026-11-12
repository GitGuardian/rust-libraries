//! Path manipulation.
//!
//! These functions do not access the filesystem unless their documentation says
//! so.

mod canonicalize;
mod case;
mod clean;
mod hash;
mod join;
mod parent;
mod relative;
mod within;

pub use canonicalize::canonicalize;
pub use case::{CaseSensitivity, case_sensitivity};
pub use clean::lexical_clean;
pub use hash::stable_hash;
pub use join::join;
pub use relative::project_relative;
pub use within::{is_within, is_within_with};

pub(crate) use parent::parent_dir;
