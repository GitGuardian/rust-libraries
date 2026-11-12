#![doc = include_str!("../README.md")]

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
compile_error!("gg_fs supports linux, macos and windows only");

mod binary;
mod dir;
mod edit;
mod error;
mod file;
mod lock;
mod metadata;
mod mode;
mod read;
mod sys;
mod trust;
mod walk;
mod write;

pub mod glob;
pub mod locations;
pub mod paths;
pub mod temp;

pub use binary::{is_binary, is_binary_bytes};
pub use dir::{DirOptions, create_dir_all, create_dir_all_with};
pub use edit::{Edit, Replacement};
pub use error::{Cause, Error, Result, ResultExt, UntrustedReason};
pub use file::{Access, File, OpenOptions};
pub use lock::FileLock;
pub use metadata::{FileType, Metadata, MetadataExt, MetadataOptions, metadata, metadata_with};
pub use mode::{Mode, set_umask};
pub use read::{
    DEFAULT_READ_CAP, ReadOptions, read, read_to_string, read_to_string_with, read_with,
};
pub use trust::Trust;
pub use walk::{DirEntry, Walk, WalkControl, WalkOptions, walk, walk_parallel, walk_with};
pub use write::{WriteOptions, write, write_with};
