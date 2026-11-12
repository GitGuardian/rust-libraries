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

`release-plz.yml` opens or updates a release pull request. It runs every Wednesday at 9:00 Paris time. A maintainer can also start it from the Actions UI. Merging the release pull request releases the bumped crates. Public crates are published to crates.io through trusted publishing. Every released crate gets a tag and a GitHub release.

Tags have the form `<crate>-v<version>`, for example `gg_fs-v0.2.0`. Tags are never moved or deleted. Consumers pin them.

### Release pull request workflow

`release-plz.yml` runs on a schedule and on `workflow_dispatch`. It does not run on push, so commits to `main` collect until the next run.

A manual run uses the `release-pr` environment. The environment accepts deployments from `main` only. A member of `ggshield-admins` approves each manual run in the Actions UI. Scheduled runs do not use the environment and need no approval.

### Release workflow

`release.yml` runs on every push to `main`. It runs CI first, then the `Release` job. The job runs `release-plz release`, which releases every crate whose version has no tag or is not on crates.io yet. It does this on any push, not only when a release pull request is merged. A failed release is therefore fixed by a new push to `main` or by a rerun of the failed job.

The `Release` job uses the `crates-io` environment. The environment accepts deployments from `main` only. A member of `ggshield-admins` approves each run in the Actions UI.

The `release tags` ruleset blocks updates and deletions of tags. It does not block creation, because the job creates tags with `GITHUB_TOKEN`, and a `GITHUB_TOKEN` cannot bypass a ruleset.

release-plz writes the `CHANGELOG.md` files. The prettier hook excludes them, because prettier rewrites the `*` emphasis that release-plz uses.

### Skipping a crate in one release

release-plz bumps a crate when any commit touches its files since the last release. To leave a crate out of one release, edit the release pull request before merging it. Revert the crate's `Cargo.toml` and `CHANGELOG.md`, and delete the changelog if release-plz created it. Revert the crate's entries in the root `Cargo.toml` and in `Cargo.lock`. `gitguardian-api` and `gitguardian-client` are skipped together.

The next run of `release-plz.yml` before the merge discards the edit. release-plz then rewrites the release pull request. The skipped crate is proposed again in every later release pull request until it is released.

### Dependabot

Workflows triggered by Dependabot read Dependabot secrets, not Actions secrets. `GITGUARDIAN_API_KEY` exists in both stores.

## Local overrides

A consumer can build against a local checkout with a `[patch]` section in its root `Cargo.toml`.

```toml
[patch."https://github.com/GitGuardian/rust-libraries"]
gg_fs = { path = "../rust-libraries/internal/gg_fs" }
```

Remove the section before committing.
