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

`delete` removes the user token of an instance from `auth_config.yaml` and from
the OS store, including a token that an earlier login left there. If the OS
store refuses the removal, a token may stay there, and the returned
`DeleteOutcome::KeptInOsStore` contains the reason.

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

`managed_instance` returns the instance of the machine token. It fails if
`machine_credential.yaml` cannot be used. The crate cannot remove a machine
token.

## OS stores

| System | User tokens    | Machine tokens  |
| ------ | -------------- | --------------- |
| macOS  | login keychain | System keychain |

A token is stored under the service `ggshield`, with the instance URL as the
account, as Python `keyring` stores it.

On macOS, only a program with a GitGuardian code signature stores user tokens in
the keychain. Another program stores them in `auth_config.yaml`, and reads a
keychain token only if the keychain allows it without a dialog.

## Reading

`read` returns one token for an instance. It returns the machine token if the
token belongs to the instance, can be read, grants the required scopes of the
request and has not expired. If the device has no machine token, it returns the
explicit token of the request, if any. Otherwise it returns the user token. A
machine token that is not returned, for example because it belongs to another
instance or cannot be read, still keeps the explicit token from being used. The
`Source` of the credential identifies the returned token.

A read fails with `Cause::NoToken` when no token is available. It fails with
`Cause::MachineTokenSkipped` when the device has a machine token that the read
does not return, and no user token is stored. The `SkipReason` states why.

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

`list` returns the stored tokens without their secrets. It returns the entries
it cannot read separately, with the reason, for example an instance with a
malformed URL or a `machine_credential.yaml` that cannot be used.

## Errors

Every operation returns a `CredentialStoreError`. It records the instance and
the file it applies to, if any, and the `Cause`.
