use gg_credential_store::gg_url::DashboardUrl;
use gg_credential_store::{
    Cause, CleartextReason, CredentialStore, InMemoryOsStore, ReadRequest, Setting, Source,
    StoreOutcome, TokenInfo,
};
use gg_fs::temp::TempDir;

/// GIVEN `GG_CONFIG_DIR` pointing to an empty directory, and
/// `GG_SYSTEM_CONFIG_DIR` to one without a machine token
/// WHEN a token is stored, listed, read, given a lifetime and deleted, first
/// with an OS store and then without one
/// THEN each operation sees the result of the one before
#[test]
fn operations_share_the_config_directory() {
    let dir = TempDir::new("credential_store").unwrap();
    // SAFETY: this binary has a single test, so no other thread reads the
    // environment while it changes.
    unsafe {
        std::env::set_var("GG_CONFIG_DIR", dir.path());
        std::env::set_var("GG_SYSTEM_CONFIG_DIR", dir.path().join("system"));
        std::env::set_var("GGSHIELD_NO_KEYRING", "1");
    }
    let instance: DashboardUrl = "https://dashboard.gitguardian.com".parse().unwrap();
    let info = TokenInfo {
        token_type: "personal_access_token".to_owned(),
        expire_at: None,
        workspace_id: 42,
        name: "my token".to_owned(),
        scopes: vec!["scan".to_owned()],
    };

    let store = CredentialStore::with_os_store(InMemoryOsStore::default()).unwrap();
    assert_eq!(store.store(&instance, "secret", &info, None).unwrap(), StoreOutcome::UserOsStore);
    assert_eq!(store.list().unwrap().len(), 1);
    let credential = store.read(&instance, &ReadRequest::default()).unwrap();
    assert_eq!(
        (credential.token.as_str(), credential.source),
        ("secret", Source::UserOsStore { machine_token_skipped: None })
    );
    let setting =
        Setting::DefaultTokenLifetime { instance: Some(instance.clone()), days: Some(30) };
    store.set_setting(setting).unwrap();
    assert_eq!(store.default_token_lifetime(Some(&instance)).unwrap(), Some(30));
    store.delete(&instance).unwrap();
    let err = store.read(&instance, &ReadRequest::default()).unwrap_err();
    assert!(matches!(err.cause(), Cause::NoToken), "{err}");

    let store = CredentialStore::new().unwrap();
    let outcome = store.store(&instance, "secret", &info, None).unwrap();
    let path = dir.path().join("auth_config.yaml");
    assert_eq!(outcome, StoreOutcome::UserFile { reason: CleartextReason::Disabled, path });
    let credential = store.read(&instance, &ReadRequest::default()).unwrap();
    assert_eq!(
        (credential.token.as_str(), credential.source),
        ("secret", Source::UserFile { machine_token_skipped: None })
    );
}
