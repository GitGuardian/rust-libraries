# gg_credential_store_test_client

This crate is a command-line test client for `gg_credential_store`. The harness
in `harness/` signs copies of it with different identities and checks how the
macOS keychain treats each of them. The scripts in `ci/` run it in CI. The crate
is not published.

## Test client

Each operation prints one JSON object on standard output.

| Operation                                                    | Result                                                      |
| ------------------------------------------------------------ | ----------------------------------------------------------- |
| `store --instance <url> [--interactive] [--machine-id <id>]` | `location`, and `reason` and `detail` for a cleartext token |
| `read --instance <url>`                                      | `token`, `source` and `machine_id`                          |
| `delete --instance <url>`                                    | `deleted`                                                   |
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
| 1    | any other failure       |

## CI

`ci/secret_service_linux.sh <test client>` runs in a D-Bus session. It checks
that a user token stored by the test client can be read with Python `keyring`,
and the reverse. It also checks the fallback to the config file without a
session bus and with a locked keyring.
`ci/machine_store_linux.sh <test client>` stores a machine token as root and
checks that the calling user reads it and cannot replace it.
`ci/machine_store_windows.ps1 <test client>` does the same as an administrator,
with a temporary standard user. `ci/machine_store_macos.sh <test client>` checks
that a test client without the GitGuardian signature cannot store a machine
token, and reads one from the file. `ci/keychain_macos.sh` creates a
self-signed code signing certificate, signs the test clients with it through
`rcodesign`, and runs the user and machine token cases of the harness. The
`gg_credential_store` workflow runs all five.

## Harness

### Prerequisites

- The Developer ID certificate of GitGuardian in the login keychain.
  `security find-identity -v -p codesigning` lists it. `GG_SIGN_IDENTITY` holds
  its name. `GG_SIGN_IDENTITY=-` signs ad hoc instead. This checks the harness
  itself, and the B and P cases then fail.
- `GG_KEYCHAIN_PASSWORD`, the login password, adds the locked keychain case.

### Phases

```bash
internal/gg_credential_store_test_client/harness/matrix.sh build
internal/gg_credential_store_test_client/harness/matrix.sh user
internal/gg_credential_store_test_client/harness/matrix.sh machine
internal/gg_credential_store_test_client/harness/matrix.sh cleanup
```

| Phase     | Runs as                 | Effect                                                                                                                                                                                       |
| --------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `build`   | you                     | builds the test client twice and signs copies into `harness/bin`: `v1` and `v2`, each with `ggshield` and `ggshield-py`, a `relocated` copy of `v1/ggshield`, and an ad hoc `adhoc/ggshield` |
| `user`    | you, in the GUI session | runs the `B`, `P` and `C` cases below                                                                                                                                                        |
| `machine` | you, and root with sudo | runs the `M` cases below                                                                                                                                                                     |
| `cleanup` | you                     | removes the test items from the login keychain, and the machine token of the `M` cases                                                                                                       |

The signed copies carry the identifiers `com.gitguardian.ggshield` and
`com.gitguardian.ggshield-py`. The build phase checks them against the
requirement files of `gg_credential_store`.

The cases use the service `ggshield` and the instances
`https://b.test-client.invalid`, `https://p.test-client.invalid` and
`https://c.test-client.invalid`. `GG_CONFIG_DIR` and `GG_CACHE_DIR` point to a
temporary directory, so the `auth_config.yaml` of the user is not changed.

The `M` cases use the instance `https://m.test-client.invalid`, the System
keychain and `/Library/Application Support/ggshield/machine_credential.yaml`.
The machine phase refuses to run if that directory exists, and removes the
machine token at the end.

Each case writes one line to `harness/results/<timestamp>-<user>.tsv`: the
label, the expected kind and exit code, the actual kind and exit code, whether
`SecurityAgent` showed a dialog while the case ran, and the verdict. A dialog
turns any verdict into `dialog`.

### Cases

| Label                 | Test client          | Operation               | Expected                     |
| --------------------- | -------------------- | ----------------------- | ---------------------------- |
| `B-self-store`        | `v1/ggshield`        | store for b             | `user_os_store`              |
| `B-self-read`         | `v1/ggshield`        | read b                  | `user_os_store`              |
| `B-relocated-read`    | `relocated/ggshield` | read b                  | `user_os_store`              |
| `B-upgraded-read`     | `v2/ggshield`        | read b                  | `user_os_store`              |
| `B-py-read`           | `v1/ggshield-py`     | read b                  | `user_os_store`              |
| `B-py-upgraded-read`  | `v2/ggshield-py`     | read b                  | `user_os_store`              |
| `B-adhoc-read`        | `adhoc/ggshield`     | read b                  | `os_store_denied`, no dialog |
| `B-locked-read`       | `v1/ggshield`        | read b, keychain locked | `os_store_locked`            |
| `B-delete`            | `v1/ggshield`        | delete b                | success                      |
| `B-read-after-delete` | `v1/ggshield`        | read b                  | `no_token`                   |
| `P-self-store`        | `v1/ggshield-py`     | store for p             | `user_os_store`              |
| `P-ggshield-read`     | `v1/ggshield`        | read p                  | `user_os_store`              |
| `P-delete`            | `v1/ggshield-py`     | delete p                | success                      |
| `C-self-store`        | `adhoc/ggshield`     | store for c             | `user_file`                  |
| `C-self-read`         | `adhoc/ggshield`     | read c                  | `user_file`                  |
| `C-signed-read`       | `v1/ggshield`        | read c                  | `user_file`                  |
| `C-delete`            | `adhoc/ggshield`     | delete c                | success                      |
| `M-user-store`        | `v1/ggshield`        | store for m             | `machine_store_denied`       |
| `M-root-store`        | `v1/ggshield`, root  | store for m             | `machine_os_store`           |
| `M-root-read`         | `v1/ggshield`, root  | read m                  | `machine_os_store`           |
| `M-self-read`         | `v1/ggshield`        | read m                  | `machine_os_store`           |
| `M-relocated-read`    | `relocated/ggshield` | read m                  | `machine_os_store`           |
| `M-upgraded-read`     | `v2/ggshield`        | read m                  | `machine_os_store`           |
| `M-py-read`           | `v1/ggshield-py`     | read m                  | `machine_os_store`           |
| `M-adhoc-read`        | `adhoc/ggshield`     | read m                  | `no_token`                   |
| `M-managed-instance`  | `adhoc/ggshield`     | managed instance        | m                            |

### Partition list

Items in the login keychain carry a partition list. It holds the team
identifier of the program that created the item, or its code hash when the
program has no team identifier. Without a team identifier, a binary other than
the creator is refused, even with a correct access list. `ggshield` and
`ggshield-py` have different code hashes, as do two builds. `B-upgraded-read`,
`B-py-read`, `B-py-upgraded-read` and `P-ggshield-read` therefore pass only
with the Developer ID identity. With a self-signed identity, the harness
expects `os_store_denied` for them. Items in the System keychain are read
through their access list alone, so `M-upgraded-read` and `M-py-read` pass with
both identities.

### Self-signed identity

A build with `RUSTFLAGS="--cfg gg_credential_store_test_signer"` accepts, in
place of the GitGuardian requirement, a program signed with the certificate
whose SHA-1 hash `GG_CREDENTIAL_STORE_TEST_SIGNER` holds at compile time.
`GG_SIGN_IDENTITY` then names a PEM file with the key and the certificate, and
`rcodesign` signs with it. `ci/keychain_macos.sh` sets all three.

### Self-test

`harness/test_lib.sh` checks the recording of results with `/usr/bin/true`,
`/usr/bin/false`, a command that exceeds the timeout, and canned JSON output.
