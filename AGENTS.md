# rust-libraries

This repository is a Cargo workspace. It holds the Rust libraries of GitGuardian.

## Layout

Crates under `public/` are published to crates.io. Crates under `internal/` are not published. Consumers use them as git dependencies pinned by tag.

The root `Cargo.toml` holds the shared package fields, lints and dependencies. Each crate inherits them with `.workspace = true` and sets `[lints] workspace = true`.

## Rules

A public crate depends only on public crates. A public crate may use an internal crate as a dev dependency without a version. Cargo removes such dependencies at publish time.

An internal crate may depend on public or internal crates. Each internal crate sets `publish = false`.

`gitguardian-client` requires `gitguardian-api` with an `=` version. The two crates ship together. `release-plz.toml` puts them in one version group.

CI runs `cargo publish --workspace --dry-run`. It fails when a public crate depends on an internal one.

## Tooling

The toolchain is pinned in `rust-toolchain.toml`. Formatting uses nightly rustfmt with the root `rustfmt.toml`.

```bash
cargo +nightly fmt --all
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
```

The end to end tests of `gitguardian-client` start `gitguardian-mock`. It needs Node.js and runs on Unix only.

## Adding a crate

Create the crate under `public/` or `internal/`. The workspace picks it up through the `members` globs.

Inherit `edition`, `rust-version`, `license`, `repository` and `homepage` from the workspace. Set `[lints] workspace = true`. Give the crate its own `version`, a `description` and a `README.md`.

For an internal crate, set `publish = false` in `Cargo.toml`. Add a `[[package]]` entry with `git_only = true` to `release-plz.toml`.

For a public crate, publish the first version by hand. Then configure the crates.io trusted publisher for `release.yml`.

## Releasing

Commits follow conventional commits with the crate name as scope, for example `feat(gg_fs): add walk_parallel`. release-plz reads them to pick the version bump and write each crate's `CHANGELOG.md`.

On every push to `main`, release-plz opens or updates a release pull request. Merging it releases the bumped crates. Public crates are published to crates.io through trusted publishing. Every released crate gets a tag and a GitHub release.

Tags have the form `<crate>-v<version>`, for example `gg_fs-v0.2.0`. Tags are never moved or deleted. Consumers pin them.

## Local overrides

A consumer can build against a local checkout with a `[patch]` section in its root `Cargo.toml`.

```toml
[patch."https://github.com/GitGuardian/rust-libraries"]
gg_fs = { path = "../rust-libraries/internal/gg_fs" }
```

Remove the section before committing.
