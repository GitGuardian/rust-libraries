use std::sync::{Mutex, MutexGuard, PoisonError};

use super::security_sys;
use crate::Cause;

static SERIAL: Mutex<()> = Mutex::new(());

/// Sets whether the keychain may show dialogs in this process while it is
/// alive.
#[derive(Debug)]
#[must_use = "the setting applies only while the guard is alive"]
pub(super) struct KeychainDialogs {
    _serial: MutexGuard<'static, ()>,
    previous: bool,
}

impl KeychainDialogs {
    /// Forbids keychain dialogs until the guard is dropped. A thread must not
    /// hold two guards.
    pub(super) fn forbid() -> Result<Self, Cause> {
        Self::set(false)
    }

    /// Allows keychain dialogs until the guard is dropped. A thread must not
    /// hold two guards.
    pub(super) fn allow() -> Result<Self, Cause> {
        Self::set(true)
    }

    fn set(allowed: bool) -> Result<Self, Cause> {
        let serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
        let previous = security_sys::user_interaction_allowed()?;
        security_sys::set_user_interaction_allowed(allowed)?;
        Ok(Self { _serial: serial, previous })
    }
}

impl Drop for KeychainDialogs {
    fn drop(&mut self) {
        if let Err(cause) = security_sys::set_user_interaction_allowed(self.previous) {
            tracing::warn!("the keychain dialog setting cannot be restored: {cause}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::*;

    /// GIVEN a guard that forbids or allows dialogs
    /// WHEN the setting is read while it is held
    /// THEN it is the one the guard set
    #[test]
    fn guard_sets_dialogs() {
        let forbidden = KeychainDialogs::forbid().unwrap();
        let allowed_while_forbidden = security_sys::user_interaction_allowed().unwrap();
        drop(forbidden);
        let allowed = KeychainDialogs::allow().unwrap();
        let allowed_while_allowed = security_sys::user_interaction_allowed().unwrap();
        drop(allowed);

        assert!(!allowed_while_forbidden);
        assert!(allowed_while_allowed);
    }

    /// GIVEN a guard that is dropped
    /// WHEN the next guard is created
    /// THEN it finds the setting the first guard found
    #[test]
    fn guard_restores_setting_it_found() {
        let first = KeychainDialogs::forbid().unwrap();
        let found = first.previous;
        drop(first);

        let second = KeychainDialogs::allow().unwrap();

        assert_eq!(second.previous, found);
    }

    /// GIVEN a guard held by one thread
    /// WHEN another thread creates a guard
    /// THEN it waits until the first guard is dropped
    #[test]
    fn second_guard_waits_for_first() {
        let first = KeychainDialogs::forbid().unwrap();
        let (sender, receiver) = mpsc::channel();
        let waiting = thread::spawn(move || {
            let _second = KeychainDialogs::forbid().unwrap();
            sender.send(()).unwrap();
        });

        let before_drop = receiver.recv_timeout(Duration::from_millis(100));
        drop(first);
        let after_drop = receiver.recv_timeout(Duration::from_secs(5));
        waiting.join().unwrap();

        assert!(before_drop.is_err());
        assert!(after_drop.is_ok());
    }
}
