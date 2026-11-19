use super::Scope;
use crate::{Cause, CleartextReason};

/// The OS store of a platform without one.
#[derive(Debug)]
pub(crate) enum PlatformStore {}

impl PlatformStore {
    pub(crate) fn current() -> Option<Self> {
        None
    }

    pub(crate) fn check_available(&self, _scope: Scope) -> Result<(), CleartextReason> {
        match *self {}
    }

    pub(crate) fn read(&self, _scope: Scope, _account: &str) -> Result<Option<String>, Cause> {
        match *self {}
    }

    pub(crate) fn write(
        &self,
        _scope: Scope,
        _account: &str,
        _token: &str,
        _interactive: bool,
    ) -> Result<(), Cause> {
        match *self {}
    }

    pub(crate) fn delete(&self, _account: &str) -> Result<(), Cause> {
        match *self {}
    }
}
