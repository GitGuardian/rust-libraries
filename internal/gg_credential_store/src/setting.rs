use gg_url::DashboardUrl;

/// A setting changed by [`CredentialStore::set_setting`](crate::CredentialStore::set_setting).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Setting {
    /// The lifetime in days of new tokens. `None` removes the setting.
    DefaultTokenLifetime {
        /// The instance to change, or `None` for the global setting.
        instance: Option<DashboardUrl>,
        /// The lifetime in days.
        days: Option<u32>,
    },
}
