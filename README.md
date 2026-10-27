# rust-libraries

This repository holds the Rust libraries of GitGuardian.

Public crates are published to crates.io. Internal crates are not published. Other projects use them as git dependencies pinned by tag.

| Crate                                           | Kind     | Description                                            |
| ----------------------------------------------- | -------- | ------------------------------------------------------ |
| [gitguardian-api](public/gitguardian-api)       | public   | Request definitions and models for the GitGuardian API |
| [gitguardian-client](public/gitguardian-client) | public   | GitGuardian API client with ureq and reqwest backends  |
| [gitguardian-mock](internal/gitguardian-mock)   | internal | Local mock of the GitGuardian API for end to end tests |

`gitguardian-mock` is only used by the tests of this repository. It is not tagged.

## Using a public crate

Add the crate from crates.io.

```bash
cargo add gitguardian-client
```

## Using an internal crate

Internal crates are geared towards being used in other projects of the gitguardian codebase.
Support for external feature requests and bug investigation is limited.
Add the crate as a git dependency. The tag names the crate and its version.

```toml
gg_fs = { git = "https://github.com/GitGuardian/rust-libraries", tag = "gg_fs-v0.1.0" }
```

To upgrade, change the tag. Internal crates may change their API in any minor version.

## License

The crates are released under the [MIT license](LICENSE).
