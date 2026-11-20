# gg_credential_store_test_client

This crate is a command-line test client for `gg_credential_store`. The scripts
in `ci/` run it in CI against the OS stores of each system. The crate is not
published.

## Test client

Each operation prints one JSON object on standard output.

| Operation                                                    | Result                                                      |
| ------------------------------------------------------------ | ----------------------------------------------------------- |
| `store --instance <url> [--interactive] [--machine-id <id>]` | `location`, and `reason` and `detail` for a cleartext token |
| `read --instance <url>`                                      | `token`, `source` and `machine_id`                          |
| `delete --instance <url>`                                    | `outcome`, and `reason` and `detail` for a token kept       |
| `managed-instance`                                           | `instance`                                                  |

`store` reads the token from standard input. A failure prints `error.kind` and
exits with one of these codes:

| Code | Kind                    |
| ---- | ----------------------- |
| 2    | `os_store_denied`       |
| 3    | `os_store_item_missing` |
| 4    | `os_store_locked`       |
| 5    | `os_store_unavailable`  |
| 6    | `no_token`              |
| 7    | `machine_store_denied`  |
| 8    | `missing_machine_id`    |
| 9    | `machine_token_skipped` |
| 1    | any other failure       |

## CI

`ci/secret_service_linux.sh <test client>` runs in a D-Bus session. It checks
that a user token stored by the test client can be read with Python `keyring`,
and the reverse. It also checks the fallback to the config file without a
session bus and with a locked keyring.
`ci/credential_manager_windows.ps1 <test client>` checks the same exchange with
Python `keyring` in the Credential Manager. It includes tokens that either side
moved to their compound target.
`ci/machine_store_linux.sh <test client>` stores a machine token as root and
checks that the calling user reads it and cannot replace it.
`ci/machine_store_windows.ps1 <test client>` does the same as an administrator,
with a temporary standard user. `ci/machine_store_macos.sh <test client>` checks
that a test client without the GitGuardian signature cannot store a machine
token, and reads one from the file. `ci/keychain_macos.sh` runs the keychain
cases below. The `gg_credential_store` workflow runs all six.

## Keychain cases

`ci/keychain_macos.sh` changes the login keychain and the System keychain of the
user who runs it, so it runs in CI only. It needs `rcodesign`, `jq` and `sudo`
without a password.

The script creates a self-signed code signing certificate that is valid for one
day. It builds the test client twice with
`RUSTFLAGS="--cfg gg_credential_store_test_signer"`, and signs copies with
`rcodesign`: `v1` and `v2`, each as `ggshield` (`com.gitguardian.ggshield`) and
as `ggshield-py` (`com.gitguardian.ggshield-py`). It adds a relocated copy of
`v1/ggshield` and an ad hoc signed `adhoc/ggshield`. With the `cfg`, the crate
accepts a program signed with the certificate whose SHA-1 hash
`GG_CREDENTIAL_STORE_TEST_SIGNER` holds at compile time, in place of the
GitGuardian requirement. The production requirement files of
`gg_credential_store` are therefore not exercised here.

The cases use the service `ggshield` and instances under `test-client.invalid`.
`GG_CONFIG_DIR` and `GG_CACHE_DIR` point to a temporary directory. The `M` cases
use the System keychain and
`/Library/Application Support/ggshield/machine_credential.yaml`. The script
refuses to run if that directory exists, and removes the test items and the
machine token at the end.

A case passes if the test client returns the expected result or error kind
within 20 seconds, and no `SecurityAgent` process started while it ran. A new
`SecurityAgent` process counts as a dialog and fails the case.

| Label                 | Test client          | Operation        | Expected                |
| --------------------- | -------------------- | ---------------- | ----------------------- |
| `B-self-store`        | `v1/ggshield`        | store for b      | `user_os_store`         |
| `B-self-read`         | `v1/ggshield`        | read b           | `user_os_store`         |
| `B-relocated-read`    | `relocated/ggshield` | read b           | `user_os_store`         |
| `B-upgraded-read`     | `v2/ggshield`        | read b           | `os_store_denied`       |
| `B-py-read`           | `v1/ggshield-py`     | read b           | `os_store_denied`       |
| `B-py-upgraded-read`  | `v2/ggshield-py`     | read b           | `os_store_denied`       |
| `B-adhoc-read`        | `adhoc/ggshield`     | read b           | `os_store_denied`       |
| `B-delete`            | `v1/ggshield`        | delete b         | `removed`               |
| `B-read-after-delete` | `v1/ggshield`        | read b           | `no_token`              |
| `P-self-store`        | `v1/ggshield-py`     | store for p      | `user_os_store`         |
| `P-ggshield-read`     | `v1/ggshield`        | read p           | `os_store_denied`       |
| `P-delete`            | `v1/ggshield-py`     | delete p         | `removed`               |
| `C-self-store`        | `adhoc/ggshield`     | store for c      | `user_file`             |
| `C-self-read`         | `adhoc/ggshield`     | read c           | `user_file`             |
| `C-signed-read`       | `v1/ggshield`        | read c           | `user_file`             |
| `C-delete`            | `adhoc/ggshield`     | delete c         | `removed`               |
| `M-user-store`        | `v1/ggshield`        | store for m      | `machine_store_denied`  |
| `M-root-store`        | `v1/ggshield`, root  | store for m      | `machine_os_store`      |
| `M-root-read`         | `v1/ggshield`, root  | read m           | `machine_os_store`      |
| `M-self-read`         | `v1/ggshield`        | read m           | `machine_os_store`      |
| `M-relocated-read`    | `relocated/ggshield` | read m           | `machine_os_store`      |
| `M-upgraded-read`     | `v2/ggshield`        | read m           | `machine_os_store`      |
| `M-py-read`           | `v1/ggshield-py`     | read m           | `machine_os_store`      |
| `M-adhoc-read`        | `adhoc/ggshield`     | read m           | `machine_token_skipped` |
| `M-managed-instance`  | `adhoc/ggshield`     | managed instance | m                       |

### Partition list

Items in the login keychain carry a partition list. It holds the team
identifier of the program that created the item, or its code hash when the
program has no team identifier. A self-signed certificate has no team
identifier, so a binary other than the creator is refused, even with a correct
access list. `ggshield` and `ggshield-py` have different code hashes, as do two
builds. `B-upgraded-read`, `B-py-read`, `B-py-upgraded-read` and
`P-ggshield-read` therefore expect `os_store_denied`. Signed with GitGuardian's
Developer ID, these binaries share the team identifier and read the item. Only a
test with that certificate can check this, and its key is not available here.
Items in the System keychain are read through their access list alone, so
`M-upgraded-read` and `M-py-read` read the token.
