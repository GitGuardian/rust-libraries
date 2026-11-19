# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/GitGuardian/rust-libraries/compare/gg_fs-v0.3.0...gg_fs-v0.4.0) - 2026-11-19

### Added

- [**breaking**] add the world-readable mode

## [0.3.0](https://github.com/GitGuardian/rust-libraries/compare/gg_fs-v0.2.0...gg_fs-v0.3.0) - 2026-11-18

### Added

- [**breaking**] add the system configuration directory

## [0.2.0](https://github.com/GitGuardian/rust-libraries/compare/gg_fs-v0.1.2...gg_fs-v0.2.0) - 2026-11-16

### Fixed

- [**breaking**] create temporary entries privately in the system temp directory

### Other

- move the Windows security identifier and token to sys

## [0.1.2](https://github.com/GitGuardian/rust-libraries/compare/gg_fs-v0.1.1...gg_fs-v0.1.2) - 2026-11-13

### Added

- implement common traits for gg_fs public types

## [0.1.1](https://github.com/GitGuardian/rust-libraries/compare/gg_fs-v0.1.0...gg_fs-v0.1.1) - 2026-11-12

### Fixed

- keep edits away from neighboring files
- accept a link to an owned folder in `create_dir_all`

## [0.1.0](https://github.com/GitGuardian/rust-libraries/releases/tag/gg_fs-v0.1.0) - 2026-11-10

### Added

- *(gg_fs)* add internal gg_fs crate
