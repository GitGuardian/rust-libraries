# rust-libraries

This repository holds the Rust libraries of GitGuardian.

Public crates are published to crates.io. Internal crates are not published to
crates.io.

| Crate                                               | Kind     | Description                                            |
| --------------------------------------------------- | -------- | ------------------------------------------------------ |
| [gitguardian-api](public/gitguardian-api)           | public   | Request definitions and models for the GitGuardian API |
| [gitguardian-client](public/gitguardian-client)     | public   | GitGuardian API client with ureq and reqwest backends  |
| [gg_url](public/gg_url)                             | public   | URL handling for GitGuardian products                  |
| [gg_fs](public/gg_fs)                               | public   | Filesystem operations with secure, portable defaults   |
| [gg_credential_store](internal/gg_credential_store) | internal | Storage of GitGuardian tokens on a device              |
| [gitguardian-mock](internal/gitguardian-mock)       | internal | Local mock of the GitGuardian API for end to end tests |

`gg_credential_store` is released through git tags of the form
`gg_credential_store-v<version>`. `gitguardian-mock` is only used by the tests
of this repository. It is not tagged.

## Using a crate

Add a public crate from crates.io.

```bash
cargo add gitguardian-client
```

Add `gg_credential_store` from this repository, pinned to a tag.

```bash
cargo add gg_credential_store \
  --git https://github.com/GitGuardian/rust-libraries \
  --tag gg_credential_store-v0.1.0
```

## License

The crates are released under the [MIT license](LICENSE).
