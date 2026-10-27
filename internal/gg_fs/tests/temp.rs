mod support;

use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;

use gg_fs::Cause;
use gg_fs::temp::{TempDir, TempFile};

/// GIVEN a scratch dir
/// WHEN it is created and dropped
/// THEN it is 0700 while alive and gone afterwards
#[test]
fn temp_dir_is_private_and_removed_on_drop() {
    let dir = TempDir::new("temp-test-private").unwrap();
    let path = dir.path().to_path_buf();
    std::fs::write(path.join("inner.txt"), "x").unwrap();

    assert!(path.is_dir());
    #[cfg(unix)]
    assert_eq!(support::unix::mode_of(&path), 0o700);
    drop(dir);

    assert!(!path.exists());
}

/// GIVEN a guard whose path is taken
/// WHEN the guard is dropped
/// THEN the directory remains
#[test]
fn temp_dir_can_be_kept_by_taking_its_path() {
    let dir = TempDir::new("temp-test-kept").unwrap();
    let path = dir.into_path();

    assert!(path.is_dir());
    std::fs::remove_dir_all(&path).unwrap();
}

/// GIVEN a scratch dir in a panicking scope
/// WHEN the panic unwinds
/// THEN the directory is removed
#[test]
fn temp_dir_is_removed_when_unwinding_a_panic() {
    let mut path = PathBuf::new();

    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let dir = TempDir::new("temp-test-unwind").unwrap();
        path = dir.path().to_path_buf();
        panic!("unwinding");
    }));

    assert!(result.is_err());
    assert!(!path.as_os_str().is_empty());
    assert!(!path.exists());
}

/// GIVEN names with separators, dotdot or nothing
/// WHEN a scratch dir or file is requested
/// THEN an invalid-path error is returned and nothing is created
#[test]
fn temp_names_that_are_not_a_single_file_name_are_rejected() {
    for name in ["../escape", "a/b", "a\\b", "..", ""] {
        let err = TempDir::new(name).err().unwrap();
        assert!(matches!(err.cause(), Cause::InvalidPath(_)), "{name:?}");
    }
    for prefix in ["../escape-", "a/b-"] {
        let err = TempFile::new(prefix).err().unwrap();
        assert!(matches!(err.cause(), Cause::InvalidPath(_)), "{prefix:?}");
    }
}

/// GIVEN an existing directory with the same name
/// WHEN a fresh one is created
/// THEN the old contents are gone
#[test]
fn temp_dir_replaces_stale_directory_of_same_name() {
    let stale = TempDir::new("temp-test-stale").unwrap();
    std::fs::write(stale.path().join("old.txt"), "old").unwrap();
    let path = stale.into_path();

    let fresh = TempDir::new("temp-test-stale").unwrap();

    assert_eq!(fresh.path(), path);
    assert!(!path.join("old.txt").exists());
}

/// GIVEN a fresh host
/// WHEN a scratch dir is created
/// THEN its parent is the root named with the uid and is 0700
#[cfg(unix)]
#[test]
fn temp_root_is_per_uid_and_private() {
    let dir = TempDir::new("temp-test-root").unwrap();
    let root = dir.path().parent().unwrap();

    let expected = format!("gg_fs-{}", support::unix::euid());
    assert_eq!(root.file_name().unwrap().to_str().unwrap(), expected);
    assert_eq!(support::unix::mode_of(root), 0o700);
}

/// GIVEN a Windows host
/// WHEN a scratch dir is created
/// THEN its parent is a gg_fs subdirectory of the system temp dir
#[cfg(windows)]
#[test]
fn windows_temp_root_uses_a_subdirectory() {
    let dir = TempDir::new("temp-test-root").unwrap();
    let root = dir.path().parent().unwrap();

    assert_eq!(root.file_name().unwrap(), "gg_fs");
    assert_eq!(
        gg_fs::paths::canonicalize(root.parent().unwrap()).unwrap(),
        gg_fs::paths::canonicalize(std::env::temp_dir()).unwrap()
    );
}

/// GIVEN a temp file request
/// WHEN it is created
/// THEN it is new, 0600, in the private root and open for reading and writing
#[test]
fn temp_file_is_created_exclusively_with_0600() {
    let root = TempDir::new("temp-test-file-root").unwrap();
    let mut file = TempFile::new("spill-").unwrap();

    assert!(file.path().file_name().unwrap().to_str().unwrap().starts_with("spill-"));
    assert_eq!(file.path().parent(), root.path().parent());
    #[cfg(unix)]
    assert_eq!(support::unix::mode_of(file.path()), 0o600);
    file.write_all(b"spilled").unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut back = String::new();
    file.read_to_string(&mut back).unwrap();
    assert_eq!(back, "spilled");
}

/// GIVEN a temp file guard
/// WHEN it is dropped
/// THEN the file is removed
#[test]
fn temp_file_is_removed_on_drop() {
    let file = TempFile::new("gone-").unwrap();
    let path = file.path().to_path_buf();
    assert!(path.is_file());

    drop(file);

    assert!(!path.exists());
}

/// GIVEN many temp files created at once
/// WHEN their paths are compared
/// THEN every name is unique
#[test]
fn temp_file_names_are_unique() {
    let files: Vec<TempFile> = (0..50).map(|_| TempFile::new("same-").unwrap()).collect();

    let paths: HashSet<&std::path::Path> = files.iter().map(TempFile::path).collect();

    assert_eq!(paths.len(), files.len());
}
