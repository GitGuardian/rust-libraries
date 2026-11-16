mod support;

use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;

use gg_fs::Cause;
use gg_fs::paths::canonicalize;
use gg_fs::temp::{TempDir, TempFile};

/// GIVEN a scratch dir
/// WHEN it is created and dropped
/// THEN it is 0700 while alive and gone afterwards
#[test]
fn temp_dir_is_private_and_removed_on_drop() {
    let dir = TempDir::new("temp-test-private-").unwrap();
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
    let dir = TempDir::new("temp-test-kept-").unwrap();
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
        let dir = TempDir::new("temp-test-unwind-").unwrap();
        path = dir.path().to_path_buf();
        panic!("unwinding");
    }));

    assert!(result.is_err());
    assert!(!path.as_os_str().is_empty());
    assert!(!path.exists());
}

/// GIVEN prefixes with separators or dotdot
/// WHEN a scratch dir or file is requested
/// THEN an invalid-path error is returned and nothing is created
#[test]
fn temp_prefixes_that_do_not_start_a_single_file_name_are_rejected() {
    for prefix in ["../escape-", "a/b-", "a\\b-"] {
        let err = TempDir::new(prefix).err().unwrap();
        assert!(matches!(err.cause(), Cause::InvalidPath(_)), "{prefix:?}");
        let err = TempFile::new(prefix).err().unwrap();
        assert!(matches!(err.cause(), Cause::InvalidPath(_)), "{prefix:?}");
    }
}

/// GIVEN two scratch dirs with the same prefix
/// WHEN both are created
/// THEN each has its own path, starting with the prefix
#[test]
fn temp_dirs_with_the_same_prefix_are_distinct() {
    let first = TempDir::new("temp-test-same-").unwrap();
    let second = TempDir::new("temp-test-same-").unwrap();

    assert_ne!(first.path(), second.path());
    for dir in [&first, &second] {
        let name = dir.path().file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("temp-test-same-"), "{name}");
    }
}

/// GIVEN a scratch dir and a temp file
/// WHEN they are created
/// THEN both are directly in the system temp dir
#[test]
fn temp_entries_are_in_the_system_temp_dir() {
    let dir = TempDir::new("temp-test-location-").unwrap();
    let file = TempFile::new("temp-test-location-").unwrap();

    let temp = canonicalize(std::env::temp_dir()).unwrap();
    assert_eq!(canonicalize(dir.path().parent().unwrap()).unwrap(), temp);
    assert_eq!(canonicalize(file.path().parent().unwrap()).unwrap(), temp);
}

/// GIVEN a temp file request
/// WHEN it is created
/// THEN it is new, 0600, named with the prefix and open for reading and writing
#[test]
fn temp_file_is_created_exclusively_with_0600() {
    let mut file = TempFile::new("spill-").unwrap();

    assert!(file.path().file_name().unwrap().to_str().unwrap().starts_with("spill-"));
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

/// GIVEN a Windows host
/// WHEN a scratch dir, a temp file and a file inside the dir are created
/// THEN only the current user has access to each, and the dir and the temp
/// file do not inherit from the system temp dir
#[cfg(windows)]
#[test]
fn windows_temp_entries_are_private() {
    let dir = TempDir::new("temp-test-access-").unwrap();
    let file = TempFile::new("temp-test-access-").unwrap();
    let inner = dir.path().join("inner.txt");
    std::fs::write(&inner, "x").unwrap();

    let user = support::windows::current_user();
    for created in [dir.path(), file.path()] {
        let dacl = support::windows::dacl_of(created);
        assert!(dacl.protected, "{}: {dacl:?}", created.display());
        assert_eq!(dacl.entries, vec![user.clone()], "{}", created.display());
    }
    assert_eq!(support::windows::dacl_of(&inner).entries, vec![user]);
}
