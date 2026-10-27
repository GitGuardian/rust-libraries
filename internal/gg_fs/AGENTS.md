# gg_fs

Shared filesystem crate. The public API mirrors `std::fs`; it differs only in
the defaults documented on each item.

## Conventions

- `foo` applies the defaults; `foo_with` takes an options struct. Options are
  never positional arguments.
- Path arguments are `impl AsRef<Path>`; paths returned are owned. A function
  whose result can borrow from its input takes `&Path` / `&[u8]` / `&str` and
  returns a `Cow`.
- Contents to write are `impl AsRef<[u8]>`, as in `std::fs::write`. Bytes
  that are only inspected are `&[u8]`.
- Errors are `gg_fs::Error`, which carries the operation and the path.
- Iterators yield a `Result` per item.
- Open files are `gg_fs::File`, which remembers its path. No function takes a
  bare handle.
- Docstrings follow the `std` style: a short description, `# Errors` where the
  function can fail, and the defaults we set. They state how, not why, and do
  not name consumers.

## Source layout

Each feature is a private module named after its main function, re-exported
flat from `lib.rs`. The namespaces `glob`, `locations`, `paths` and
`temp` are public.

Code that differs per target lives in the module's target files:

- `linux.rs`, `macos.rs`, `windows.rs`, plus `unix.rs` for code shared by the
  two unixes. A module only has the files it needs: if linux and macos do not
  differ, it has `unix.rs` and `windows.rs`.
- The module's `mod.rs` declares one `mod imp;` with `#[cfg_attr(.., path =
"..")]` selecting the file for the current target, and holds the public
  items and all target-neutral logic. Target files hold small helpers that
  `mod.rs` calls through `imp`; they are not public API.
- `sys/` holds primitives used by more than one module (file identity,
  `geteuid`, passwd lookup, umask). Policy stays in the feature modules.
- Only the current target's files are compiled, so CI runs clippy and the
  tests natively on Linux, macOS and Windows.

```
file/       File, OpenOptions            unix linux macos windows
metadata/   metadata, MetadataExt        unix windows
trust/      Trust                        unix windows
dir/        create_dir_all*              unix windows
write/      write*, WriteOptions         unix windows
lock/       FileLock                     unix windows
temp/       TempDir, TempFile            unix windows
walk/       walk*, WalkOptions           linux macos windows
locations/  Locations, Dir               unix linux macos windows
paths/      lexical_clean, is_within, ... canonicalize/{unix,windows}
```

Single-file modules: `binary`, `edit`, `error`, `glob`, `mode`,
`read`.
