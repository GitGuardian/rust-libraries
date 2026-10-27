mod support;

#[cfg(unix)]
use std::sync::mpsc;
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration;

use gg_fs::{Cause, ReadOptions, Trust, create_dir_all, read_with};
#[cfg(unix)]
use gg_fs::{DirOptions, File, Mode, OpenOptions, UntrustedReason, create_dir_all_with};
use support::Scratch;

fn trusted(trust: Trust) -> ReadOptions {
    ReadOptions { trust, ..ReadOptions::default() }
}

/// GIVEN a missing directory
/// WHEN it is created
/// THEN its mode is 0700 on unix
#[cfg(unix)]
#[test]
fn create_dir_all_sets_0700() {
    let scratch = Scratch::new();
    let dir = scratch.path("private");

    create_dir_all(&dir).unwrap();

    assert_eq!(support::unix::mode_of(&dir), 0o700);
}

/// GIVEN a path with several missing ancestors
/// WHEN it is created
/// THEN every created directory is 0700 and existing ancestors are untouched
#[cfg(unix)]
#[test]
fn create_dir_all_creates_missing_parents_privately() {
    let scratch = Scratch::new();
    let existing = scratch.dir("existing");
    support::unix::set_mode(&existing, 0o755);

    create_dir_all(existing.join("a/b/c")).unwrap();

    for created in ["existing/a", "existing/a/b", "existing/a/b/c"] {
        assert_eq!(support::unix::mode_of(&scratch.path(created)), 0o700, "{created}");
    }
    assert_eq!(support::unix::mode_of(&existing), 0o755);
}

/// GIVEN an existing directory
/// WHEN it is created again
/// THEN it succeeds and its mode is unchanged
#[cfg(unix)]
#[test]
fn create_dir_all_leaves_existing_directory_alone() {
    let scratch = Scratch::new();
    let dir = scratch.dir("existing");
    support::unix::set_mode(&dir, 0o755);

    create_dir_all(&dir).unwrap();

    assert_eq!(support::unix::mode_of(&dir), 0o755);
}

/// GIVEN an exact directory mode
/// WHEN missing directories are created
/// THEN each has that mode
#[cfg(unix)]
#[test]
fn create_dir_all_with_applies_the_requested_mode() {
    let scratch = Scratch::new();
    let options = DirOptions { mode: Mode::Exact(0o750) };

    create_dir_all_with(scratch.path("a/b"), &options).unwrap();

    assert_eq!(support::unix::mode_of(&scratch.path("a")), 0o750);
    assert_eq!(support::unix::mode_of(&scratch.path("a/b")), 0o750);
}

/// GIVEN a symlink pointing at a directory
/// WHEN it is created
/// THEN an error is returned and the target mode is unchanged
#[cfg(unix)]
#[test]
fn create_dir_all_refuses_symlink_target() {
    let scratch = Scratch::new();
    let target = scratch.dir("target");
    support::unix::set_mode(&target, 0o755);
    let link = scratch.path("link");
    support::unix::link(&target, &link);

    let err = create_dir_all(&link).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(support::unix::mode_of(&target), 0o755);
}

/// GIVEN a regular file in the way
/// WHEN a directory is created there
/// THEN a not-a-directory error is returned
#[test]
fn create_dir_all_refuses_regular_file() {
    let scratch = Scratch::new();
    let file = scratch.write("file", "x");

    let err = create_dir_all(&file).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::NotADirectory);
    assert_eq!(err.path(), file);
}

/// GIVEN a symlink to a regular file
/// WHEN it is opened without following
/// THEN the open fails, while following succeeds
#[cfg(unix)]
#[test]
fn open_nofollow_rejects_symlink() {
    let scratch = Scratch::new();
    let target = scratch.write("target.txt", "x");
    let link = scratch.path("link.txt");
    support::unix::link(&target, &link);
    let nofollow = OpenOptions { follow_symlinks: false, ..OpenOptions::default() };

    let err = File::open_with(&link, &nofollow).unwrap_err();

    assert!(matches!(err.cause(), Cause::NotRegularFile));
    assert!(File::open(&link).is_ok());
}

/// GIVEN a FIFO at the path
/// WHEN it is opened
/// THEN the call returns an error instead of blocking
#[cfg(unix)]
#[test]
fn open_does_not_block_on_fifo() {
    let scratch = Scratch::new();
    let fifo = scratch.path("pipe");
    support::unix::make_fifo(&fifo);
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || sender.send(File::open(&fifo).map(drop)).unwrap());

    let result = receiver.recv_timeout(Duration::from_secs(10)).expect("the open blocked");
    assert!(matches!(result.unwrap_err().cause(), Cause::NotRegularFile));
}

/// GIVEN a regular file owned by the caller with mode 0600
/// WHEN it is read as trusted
/// THEN its contents are returned
#[cfg(unix)]
#[test]
fn trusted_read_accepts_owner_private_file() {
    let scratch = Scratch::new();
    let file = scratch.write("state", "secret");
    support::unix::set_mode(&file, 0o600);

    for trust in [Trust::Caller, Trust::RootOrHomeOwner] {
        assert_eq!(read_with(&file, &trusted(trust)).unwrap(), b"secret", "{trust:?}");
    }
}

/// GIVEN a file readable by others but not writable
/// WHEN it is read as trusted
/// THEN it is accepted
#[cfg(unix)]
#[test]
fn trusted_read_accepts_world_readable_file() {
    let scratch = Scratch::new();
    let file = scratch.write("state", "public");
    support::unix::set_mode(&file, 0o644);

    assert_eq!(read_with(&file, &trusted(Trust::Caller)).unwrap(), b"public");
}

/// GIVEN files with modes 0660, 0606 and 0666
/// WHEN each is read as trusted
/// THEN each is rejected as writable
#[cfg(unix)]
#[test]
fn trusted_read_rejects_group_or_world_writable_file() {
    let scratch = Scratch::new();
    for mode in [0o660, 0o606, 0o666] {
        let file = scratch.write(&format!("state-{mode:o}"), "x");
        support::unix::set_mode(&file, mode);

        let err = read_with(&file, &trusted(Trust::Caller)).unwrap_err();

        assert!(matches!(err.cause(), Cause::Untrusted(UntrustedReason::Writable)), "{mode:o}");
    }
}

/// GIVEN a loose file and no trust requirement
/// WHEN it is read
/// THEN its contents are returned
#[cfg(unix)]
#[test]
fn unchecked_read_accepts_loose_file() {
    let scratch = Scratch::new();
    let file = scratch.write("state", "x");
    support::unix::set_mode(&file, 0o666);

    assert_eq!(read_with(&file, &trusted(Trust::Unchecked)).unwrap(), b"x");
}

/// GIVEN the root-only policy and a file owned by a normal user
/// WHEN it is read as trusted
/// THEN it is rejected
#[cfg(unix)]
#[test]
fn root_only_policy_requires_uid_zero() {
    if support::unix::is_root() {
        return;
    }
    let scratch = Scratch::new();
    let file = scratch.write("state", "x");
    support::unix::set_mode(&file, 0o600);

    let err = read_with(&file, &trusted(Trust::RootOnly)).unwrap_err();

    assert!(matches!(err.cause(), Cause::Untrusted(UntrustedReason::Owner)));
}

/// GIVEN a directory
/// WHEN it is read as trusted
/// THEN it is refused as a non-regular file
#[test]
fn trusted_read_rejects_directory() {
    let scratch = Scratch::new();
    let dir = scratch.dir("sub");

    let err = read_with(&dir, &trusted(Trust::Caller)).unwrap_err();

    assert!(matches!(err.cause(), Cause::NotRegularFile));
}
