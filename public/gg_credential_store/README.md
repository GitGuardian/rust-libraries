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
`false` or `0` keeps user tokens out of the OS store.

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

## Machine tokens

A machine token starts with `gg_mat_` and is shared by all users of the device.
Storing one requires root or administrator rights and, on macOS, a GitGuardian
code signature. Otherwise `store` fails with `Cause::MachineStoreDenied`.

A machine token is stored in the machine-wide credential store of the operating
system when it can be used. Otherwise it is written to `machine_credential.yaml`
in clear text, readable by all users. The returned `StoreOutcome::MachineFile`
contains the reason.

`store` needs the machine id sent at enrollment, and fails with
`Cause::MissingMachineId` without it. `TokenInfo` holds the expiry date, scopes,
workspace and name of the token, from the enroll or renew response or from
`GET /v1/token`. If `store` fails, revoke the token.

`managed_instance` returns the instance of the machine token. The crate cannot
remove a machine token.

## Reading

`read` returns one token for an instance. It returns the machine token if the
token can be read, grants the required scopes of the request and has not
expired. If the machine has no machine token for the instance that can be read,
it returns the explicit token of the request, if any. Otherwise it returns the
user token. The `Source` of the credential identifies the returned token.

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
