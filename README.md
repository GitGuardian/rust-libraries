# rust-libraries

This repository holds the Rust libraries of GitGuardian.

Public crates are published to crates.io. Internal crates are not published.

| Crate                                           | Kind     | Description                                            |
| ----------------------------------------------- | -------- | ------------------------------------------------------ |
| [gitguardian-api](public/gitguardian-api)       | public   | Request definitions and models for the GitGuardian API |
| [gitguardian-client](public/gitguardian-client) | public   | GitGuardian API client with ureq and reqwest backends  |
| [gg_url](public/gg_url)                         | public   | URL handling for GitGuardian products                  |
| [gg_fs](public/gg_fs)                           | public   | Filesystem operations with secure, portable defaults   |
| [gitguardian-mock](internal/gitguardian-mock)   | internal | Local mock of the GitGuardian API for end to end tests |

`gitguardian-mock` is only used by the tests of this repository. It is not tagged.

## Using a crate

Add the crate from crates.io.

```bash
cargo add gitguardian-client
```

## License

The crates are released under the [MIT license](LICENSE).
