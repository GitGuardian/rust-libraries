use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use super::Trust;
use crate::error::{Cause, Context, Error, Result, UntrustedReason};
use crate::locations::{Dir, Locations};
use crate::metadata::metadata;
use crate::sys;

const OP: &str = "trust";
const GROUP_OR_OTHER_WRITE: u32 = 0o022;

pub(super) fn verify(file: &File, path: &Path, trust: &Trust) -> Result<()> {
    let metadata = file.metadata().context(OP, path)?;
    if *trust == Trust::Unchecked {
        Ok(())
    } else if !metadata.is_file() {
        Err(Error::new(OP, path, Cause::NotRegularFile))
    } else if !accepts(trust, metadata.uid()) {
        Err(Error::new(OP, path, Cause::Untrusted(UntrustedReason::Owner)))
    } else if metadata.mode() & GROUP_OR_OTHER_WRITE != 0 {
        Err(Error::new(OP, path, Cause::Untrusted(UntrustedReason::Writable)))
    } else {
        Ok(())
    }
}

fn accepts(trust: &Trust, uid: u32) -> bool {
    let caller = sys::geteuid();
    match trust {
        Trust::Unchecked => true,
        Trust::Caller => uid == caller,
        Trust::RootOrHomeOwner => uid == caller || uid == 0 || home_owner() == Some(uid),
        Trust::RootOnly => uid == 0,
    }
}

fn home_owner() -> Option<u32> {
    let home = Locations::current().dir(Dir::Home).ok()?;
    metadata(home).ok().map(|metadata| metadata.uid())
}
