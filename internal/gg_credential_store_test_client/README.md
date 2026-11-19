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

`ci/machine_store_linux.sh <test client>` stores a machine token as root and
checks that the calling user reads it and cannot replace it.
`ci/machine_store_windows.ps1 <test client>` does the same as an administrator,
with a temporary standard user. `ci/machine_store_macos.sh <test client>` checks
that a test client without the GitGuardian signature cannot store a machine
token, and reads one from the file. The `gg_credential_store` workflow runs all
three.

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
internal/gg_credential_store_test_client/harness/matrix.sh cleanup
```

| Phase     | Runs as                 | Effect                                                                                                                                                                                       |
| --------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `build`   | you                     | builds the test client twice and signs copies into `harness/bin`: `v1` and `v2`, each with `ggshield` and `ggshield-py`, a `relocated` copy of `v1/ggshield`, and an ad hoc `adhoc/ggshield` |
| `user`    | you, in the GUI session | runs the cases below                                                                                                                                                                         |
| `cleanup` | you                     | removes the test items from the login keychain                                                                                                                                               |

The signed copies carry the identifiers `com.gitguardian.ggshield` and
`com.gitguardian.ggshield-py`. The build phase checks them against the
requirement files of `gg_credential_store`.

The cases use the service `ggshield` and the instances
`https://b.test-client.invalid`, `https://p.test-client.invalid` and
`https://c.test-client.invalid`. `GG_CONFIG_DIR` and `GG_CACHE_DIR` point to a
temporary directory, so the `auth_config.yaml` of the user is not changed.

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

The machine token cases follow with the machine token store.

### Partition list

Items in the login keychain carry a partition list. It holds the team
identifier of the program that created the item, or its code hash when the
program has no team identifier. With an ad hoc identity, `B-upgraded-read` and
`B-py-upgraded-read` therefore fail even with a correct access list. These
cases need the Developer ID identity.

### Self-test

`harness/test_lib.sh` checks the recording of results with `/usr/bin/true`,
`/usr/bin/false`, a command that exceeds the timeout, and canned JSON output.
