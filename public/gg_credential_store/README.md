# gg_credential_store

This crate reads and writes the GitGuardian tokens of a device.

```bash
cargo add gg_credential_store
```

## Storage

A user token is stored in the credential store of the operating system when it
can be used. `auth_config.yaml` then contains a marker in place of the token.
Otherwise the token is written to `auth_config.yaml` in clear text. The returned
`StoreOutcome::UserFile` contains the reason.

`auth_config.yaml` is in the ggshield config directory. `GG_CONFIG_DIR`
overrides that directory. Setting `GGSHIELD_NO_KEYRING` to any value other than
`false` or `0` disables the OS store.

```rust,no_run
use gg_credential_store::gg_url::DashboardUrl;
use gg_credential_store::{CredentialStore, ReadRequest, StoreOutcome, TokenInfo};

let store = CredentialStore::new()?;
let instance: DashboardUrl = "https://dashboard.gitguardian.com".parse()?;
let info = TokenInfo {
    token_type: "personal_access_token".to_owned(),
    expire_at: None,
    workspace_id: 42,
    name: "my token".to_owned(),
    scopes: vec!["scan".to_owned()],
};

if let StoreOutcome::UserFile { reason, path } = store.store(&instance, "token", &info, None)? {
    eprintln!("stored in clear text in {} ({reason:?})", path.display());
}
let credential = store.read(&instance, &ReadRequest::default())?;
assert_eq!(credential.token, "token");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Reading

`read` returns one token for an instance. The machine token is returned when it
grants the required scopes of the request. On a machine without a machine token
for the instance, the explicit token of the request is returned next. The user
token is returned last. The `Source` of the credential identifies the returned
token.

A read fails with `Cause::NoToken` when no token is available. It fails with
`Cause::MachineTokenSkipped` when the machine token does not grant the required
scopes or has expired, and no user token is stored.

```rust,no_run
use gg_credential_store::gg_url::DashboardUrl;
use gg_credential_store::{Cause, CredentialStore, ReadRequest, Source};

let store = CredentialStore::new()?;
let instance: DashboardUrl = "https://dashboard.gitguardian.com".parse()?;
let request = ReadRequest {
    required_scopes: vec!["scan".to_owned()],
    explicit_token: std::env::var("GITGUARDIAN_API_KEY").ok(),
    ..ReadRequest::default()
};

let credential = match store.read(&instance, &request) {
    Ok(credential) => credential,
    Err(err) if matches!(err.cause(), Cause::NoToken) => {
        eprintln!("no token is available for {instance}");
        return Ok(());
    }
    Err(err) => return Err(err.into()),
};
if let Source::MachineOsStore { machine_id } | Source::MachineFile { machine_id } =
    &credential.source
{
    println!("machine token of the machine {machine_id}");
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Errors

Every operation returns a `CredentialStoreError`. It records the instance and
the file it applies to, if any, and the `Cause`.

## Features

`in-memory-store` adds `InMemoryOsStore` and `CredentialStore::with_os_store`,
for tests. `InMemoryOsStore::failing` returns a store whose operations fail,
for example as if the OS store were locked.

`system-config-override` reads `machine_credential.yaml` from the directory in
`GG_SYSTEM_CONFIG_DIR` and accepts a file owned by the calling user, for tests.
