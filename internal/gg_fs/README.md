# gg_fs

Filesystem operations with secure, portable defaults.

The API follows `std::fs`. Functions keep their `std` names and shapes, and
differ only in the defaults documented on each item. A function `foo` applies
the defaults, and `foo_with` takes an options struct.

## Defaults

- Writes replace the file atomically.
- Writes do not follow symbolic links.
- Directories are created with mode `0o700`.
- Reads are capped at 64 MiB.
- Errors carry the operation and the path.

The crate supports Linux, macOS and Windows. Behavior that differs by platform
is documented on each item.

## Reading

`read` and `read_to_string` read a whole file. `read_with` takes a size cap and
a trust requirement.

```rust
use gg_fs::temp::TempDir;
use gg_fs::{Cause, ReadOptions, read, read_to_string, read_with, write};

let scratch = TempDir::new("readme-reading")?;
let path = scratch.path().join("notes.txt");
write(&path, "hello")?;

assert_eq!(read_to_string(&path)?, "hello");
assert_eq!(read(&path)?, b"hello");

let options = ReadOptions { cap: 4, ..ReadOptions::default() };
let err = read_with(&path, &options).unwrap_err();
assert!(matches!(err.cause(), Cause::TooLarge { size: 5, cap: 4 }));
# Ok::<(), Box<dyn std::error::Error>>(())
```

A trusted read checks the open file. It must be a regular file, not a symbolic
link, owned by an accepted user, and not writable by group or others.

```rust
use gg_fs::temp::TempDir;
use gg_fs::{ReadOptions, Trust, WriteOptions, read_with, write_with};

let scratch = TempDir::new("readme-trust")?;
let path = scratch.path().join("token");
write_with(&path, "secret", &WriteOptions::private())?;

let options = ReadOptions { trust: Trust::Caller, ..ReadOptions::default() };
assert_eq!(read_with(&path, &options)?, b"secret");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Writing

`write` creates the file or replaces its contents. Other processes observe
either the old or the new contents. An existing file keeps its mode.

```rust
use gg_fs::temp::TempDir;
use gg_fs::{WriteOptions, read_to_string, write, write_with};

let scratch = TempDir::new("readme-writing")?;
let path = scratch.path().join("config.json");

write(&path, "{}")?;

let options = WriteOptions { durable: true, skip_unchanged: true, ..WriteOptions::default() };
write_with(&path, "{\"a\": 1}", &options)?;
assert_eq!(read_to_string(&path)?, "{\"a\": 1}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

`WriteOptions::private()` creates files only the owner may access.
`WriteOptions::output()` is for a file the user asked for. A new file gets the
mode of the host umask, and an existing file is overwritten in place.

```rust
use gg_fs::temp::TempDir;
use gg_fs::{WriteOptions, write_with};

let scratch = TempDir::new("readme-presets")?;

write_with(scratch.path().join("token"), "secret", &WriteOptions::private())?;
write_with(scratch.path().join("report.json"), "{}", &WriteOptions::output())?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

A process that installs a restrictive umask calls `set_umask` first. The
umask it replaces is the host umask. The umask applies to every thread.

```rust,no_run
let host = gg_fs::set_umask(0o077);
```

## Editing

An edit replaces byte ranges of a file. It is computed against a known hash
and refused if the file has changed since. Other edits are excluded with a lock
file beside the file.

```rust
use gg_fs::temp::TempDir;
use gg_fs::{Edit, Replacement, read_to_string, write};
use sha2::{Digest, Sha256};

let scratch = TempDir::new("readme-editing")?;
let path = scratch.path().join("settings.ini");
write(&path, "key=old\n")?;

let replacement = Replacement { range: 4..7, bytes: b"new".to_vec() };
Edit::new(Sha256::digest("key=old\n").into(), vec![replacement]).apply(&path)?;

assert_eq!(read_to_string(&path)?, "key=new\n");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Directories

`create_dir_all` creates missing directories with mode `0o700` and leaves
existing ones alone. It refuses a symbolic link.

```rust
use gg_fs::temp::TempDir;
use gg_fs::{DirOptions, Mode, create_dir_all, create_dir_all_with};

let scratch = TempDir::new("readme-directories")?;

create_dir_all(scratch.path().join("a/b/c"))?;

let options = DirOptions { mode: Mode::Exact(0o755) };
create_dir_all_with(scratch.path().join("shared"), &options)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Walking

`walk` yields the regular files below a root. It respects `.gitignore` files
and skips pseudo-filesystems. Each item is a `Result`.

```rust
use gg_fs::temp::TempDir;
use gg_fs::{WalkOptions, create_dir_all, walk, walk_with, write};

let scratch = TempDir::new("readme-walking")?;
create_dir_all(scratch.path().join("src"))?;
write(scratch.path().join("src/a.rs"), "")?;
write(scratch.path().join("src/b.rs"), "")?;
write(scratch.path().join("notes.txt"), "")?;

assert_eq!(walk(scratch.path())?.count(), 3);

let options = WalkOptions {
    extensions: Some(vec!["rs".into()]),
    sorted: true,
    ..WalkOptions::default()
};
let paths: Vec<_> = walk_with(scratch.path(), &options)?
    .map(|entry| entry.map(|entry| entry.path))
    .collect::<Result<_, _>>()?;
assert_eq!(paths, [scratch.path().join("src/a.rs"), scratch.path().join("src/b.rs")]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`walk_parallel` calls a visitor on several threads. The visitor sees every
entry, including directories and errors, and decides whether to descend.

```rust
use std::sync::Mutex;

use gg_fs::temp::TempDir;
use gg_fs::{WalkControl, WalkOptions, create_dir_all, walk_parallel, write};

let scratch = TempDir::new("readme-parallel")?;
create_dir_all(scratch.path().join("vendor"))?;
write(scratch.path().join("vendor/lib.rs"), "")?;
write(scratch.path().join("main.rs"), "")?;

let found = Mutex::new(Vec::new());
walk_parallel(scratch.path(), &WalkOptions::default(), || {
    let found = &found;
    move |entry| {
        let Ok(entry) = entry else { return WalkControl::Continue };
        if entry.file_type.is_dir() && entry.path.ends_with("vendor") {
            return WalkControl::Skip;
        }
        if entry.file_type.is_file() {
            found.lock().unwrap().push(entry.path);
        }
        WalkControl::Continue
    }
})?;

assert_eq!(found.into_inner().unwrap(), [scratch.path().join("main.rs")]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Locking

A `FileLock` is an exclusive advisory lock on a file. It is released when the
value is dropped or the process exits.

```rust
use std::time::Duration;

use gg_fs::temp::TempDir;
use gg_fs::{Cause, FileLock};

let scratch = TempDir::new("readme-locking")?;
let path = scratch.path().join("state.lock");

let lock = FileLock::lock(&path, Duration::from_secs(5))?;
let busy = FileLock::lock(&path, Duration::ZERO).err().unwrap();
assert!(matches!(busy.cause(), Cause::Timeout(_)));

drop(lock);
FileLock::lock(&path, Duration::ZERO)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Temporary files

Temporary files and directories live in a private root and are removed when
dropped.

```rust
use std::io::Write;

use gg_fs::temp::{TempDir, TempFile};

let dir = TempDir::new("readme-temporary")?;
let path = dir.path().to_path_buf();

let mut file = TempFile::new("spill-")?;
file.write_all(b"data")?;
let spilled = file.path().to_path_buf();

drop(file);
drop(dir);
assert!(!spilled.exists() && !path.exists());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Paths

The functions in `paths` do not access the filesystem unless their
documentation says so.

```rust
use std::path::Path;

use gg_fs::paths::{is_within, join, lexical_clean, project_relative};
use gg_fs::temp::TempDir;

assert_eq!(lexical_clean(Path::new("/a/b/../c")), Path::new("/a/c"));
assert!(is_within("/srv/app", "/srv/app/data"));
assert!(!is_within("/srv/app", "/srv/app/../secret"));
assert_eq!(
    project_relative(Path::new("/work/app/src/main.rs"), Path::new("/work/app")),
    "src/main.rs"
);

let base = TempDir::new("readme-paths")?;
assert_eq!(join(base.path(), "sub/file.txt")?, base.path().join("sub").join("file.txt"));
assert!(join(base.path(), "../escape").is_err());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Locations

`Locations` resolves the home, configuration, cache and state directories from
an environment. An override set to an empty string counts as unset.

```rust
use gg_fs::locations::{Dir, Locations};
use gg_fs::temp::TempDir;

let scratch = TempDir::new("readme-locations")?;
let locations = Locations::from_env([("GG_CONFIG_DIR", scratch.path().as_os_str())]);

assert_eq!(locations.dir(Dir::Config)?, scratch.path());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Patterns

`glob::Patterns` matches exclusion patterns against project-relative paths. A
`*` matches within a path component and `**` across components.

```rust
use gg_fs::glob::Patterns;

let patterns = Patterns::new(["*.log", "build/", "docs/**/draft.md"])?;

assert!(patterns.matches("logs/app.log"));
assert!(patterns.matches("build/out/a.o"));
assert!(patterns.matches("docs/a/b/draft.md"));
assert!(!patterns.matches("src/main.rs"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Errors

Every error is a `gg_fs::Error`. It records the operation, the path and the
cause, and is displayed as `<operation> <path>: <cause>`.

```rust
use std::io::ErrorKind;

use gg_fs::read;
use gg_fs::temp::TempDir;

let scratch = TempDir::new("readme-errors")?;
let err = read(scratch.path().join("missing.txt")).unwrap_err();

assert_eq!(err.op(), "open");
assert_eq!(err.kind(), ErrorKind::NotFound);
assert!(err.to_string().starts_with("open "));
# Ok::<(), Box<dyn std::error::Error>>(())
```
