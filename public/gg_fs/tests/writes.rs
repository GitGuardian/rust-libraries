mod support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Barrier, Mutex};
use std::time::{Duration, SystemTime};
use std::{fs, thread};

#[cfg(unix)]
use gg_fs::Mode;
use gg_fs::{Cause, WriteOptions, read, write, write_with};
use support::Scratch;

/// GIVEN an existing file
/// WHEN new content is written atomically
/// THEN the file holds exactly the new content
#[test]
fn atomic_write_replaces_content() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "old contents that are longer");

    write(&file, "new").unwrap();

    assert_eq!(read(&file).unwrap(), b"new");
}

/// GIVEN a missing file in an existing directory
/// WHEN content is written atomically
/// THEN the file exists with that content
#[test]
fn atomic_write_creates_missing_file() {
    let scratch = Scratch::new();
    let file = scratch.path("a.txt");

    write(&file, "created").unwrap();

    assert_eq!(read(&file).unwrap(), b"created");
}

/// GIVEN a missing parent directory
/// WHEN content is written
/// THEN a not-found error naming the path is returned and nothing is created
#[test]
fn write_does_not_create_parent_directories() {
    let scratch = Scratch::new();
    let file = scratch.path("missing/a.txt");

    let err = write(&file, "x").unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert_eq!(err.path(), file);
    assert!(!scratch.path("missing").exists());
}

/// GIVEN a concurrent reader during a large write
/// WHEN the write is in progress
/// THEN the reader sees only the old or the new content
#[test]
fn atomic_write_never_exposes_partial_content() {
    const SIZE: usize = 1 << 20;
    let scratch = Scratch::new();
    let file = scratch.write("a.bin", vec![b'a'; SIZE]);
    let done = AtomicBool::new(false);

    thread::scope(|scope| {
        scope.spawn(|| {
            for round in 0..60 {
                let byte = if round % 2 == 0 { b'b' } else { b'a' };
                write(&file, vec![byte; SIZE]).unwrap();
            }
            done.store(true, Ordering::Relaxed);
        });
        while !done.load(Ordering::Relaxed) {
            let seen = fs::read(&file).unwrap();
            assert_eq!(seen.len(), SIZE);
            let first = seen.first().copied().unwrap();
            assert!(seen.iter().all(|byte| *byte == first));
        }
    });
}

/// GIVEN a successful or failed write
/// WHEN the directory is listed
/// THEN no temporary file is left behind
#[test]
fn atomic_write_leaves_no_temp_files() {
    let scratch = Scratch::new();
    let file = scratch.path("a.txt");

    write(&file, "one").unwrap();
    write(&file, "two").unwrap();
    write(scratch.path("missing/b.txt"), "x").unwrap_err();

    assert_eq!(scratch.names(""), ["a.txt"]);
}

/// GIVEN several writers in the same process and directory
/// WHEN they write the same path at once
/// THEN every write completes and the file holds one writer's content
#[test]
fn atomic_write_temp_names_do_not_collide() {
    const WRITERS: usize = 8;
    let scratch = Scratch::new();
    let file = scratch.path("a.txt");
    let barrier = Barrier::new(WRITERS);

    thread::scope(|scope| {
        for writer in 0..WRITERS {
            let (file, barrier) = (&file, &barrier);
            scope.spawn(move || {
                barrier.wait();
                for _ in 0..25 {
                    write(file, format!("writer {writer}")).unwrap();
                }
            });
        }
    });

    let content = String::from_utf8(read(&file).unwrap()).unwrap();
    assert!(content.starts_with("writer "));
    assert_eq!(scratch.names(""), ["a.txt"]);
}

/// GIVEN a symlink at the target path
/// WHEN an atomic write runs
/// THEN an error is returned and the link target is untouched
#[cfg(unix)]
#[test]
fn atomic_write_refuses_symlink_target() {
    let scratch = Scratch::new();
    let target = scratch.write("target.txt", "original");
    let link = scratch.path("link.txt");
    support::unix::link(&target, &link);

    let err = write(&link, "new").unwrap_err();

    assert!(matches!(err.cause(), Cause::NotRegularFile));
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
}

/// GIVEN a directory at the target path
/// WHEN a write runs
/// THEN a not-regular-file error is returned
#[test]
fn write_refuses_directory() {
    let scratch = Scratch::new();
    let dir = scratch.dir("sub");

    let err = write(&dir, "x").unwrap_err();

    assert!(matches!(err.cause(), Cause::NotRegularFile));
}

/// GIVEN a symlink and follow-symlinks requested
/// WHEN a write runs
/// THEN the target is written and the link is kept
#[cfg(unix)]
#[test]
fn write_follows_symlink_chain_when_asked() {
    let scratch = Scratch::new();
    let target = scratch.write("target.txt", "original");
    support::unix::link(&target, &scratch.path("first"));
    support::unix::link(&scratch.path("first"), &scratch.path("second"));
    let options = WriteOptions { follow_symlinks: true, ..WriteOptions::default() };

    write_with(scratch.path("second"), "through links", &options).unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), "through links");
    assert!(fs::symlink_metadata(scratch.path("first")).unwrap().is_symlink());
    assert!(fs::symlink_metadata(scratch.path("second")).unwrap().is_symlink());
}

/// GIVEN a dangling symlink and follow-symlinks requested
/// WHEN a write runs
/// THEN the target is created and the link is kept
#[cfg(unix)]
#[test]
fn write_creates_dangling_symlink_target_when_following() {
    let scratch = Scratch::new();
    support::unix::link(&scratch.path("created.txt"), &scratch.path("link"));
    let options = WriteOptions { follow_symlinks: true, ..WriteOptions::default() };

    write_with(scratch.path("link"), "made", &options).unwrap();

    assert_eq!(fs::read_to_string(scratch.path("created.txt")).unwrap(), "made");
    assert!(fs::symlink_metadata(scratch.path("link")).unwrap().is_symlink());
}

/// GIVEN a symlink loop and follow-symlinks requested
/// WHEN a write runs
/// THEN an error is returned instead of hanging
#[cfg(unix)]
#[test]
fn write_reports_symlink_loops() {
    let scratch = Scratch::new();
    support::unix::link(&scratch.path("b"), &scratch.path("a"));
    support::unix::link(&scratch.path("a"), &scratch.path("b"));
    let options = WriteOptions { follow_symlinks: true, ..WriteOptions::default() };

    let err = write_with(scratch.path("a"), "x", &options).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

/// GIVEN the private options
/// WHEN a new file is written
/// THEN it has mode 0600
#[cfg(unix)]
#[test]
fn private_write_creates_file_with_0600() {
    let scratch = Scratch::new();
    let file = scratch.path("secret");

    write_with(&file, "token", &WriteOptions::private()).unwrap();

    assert_eq!(support::unix::mode_of(&file), 0o600);
}

/// GIVEN an existing file with mode 0644
/// WHEN it is written with the private options
/// THEN it ends up 0600
#[cfg(unix)]
#[test]
fn private_write_tightens_existing_loose_file() {
    let scratch = Scratch::new();
    let file = scratch.write("secret", "old");
    support::unix::set_mode(&file, 0o644);

    write_with(&file, "token", &WriteOptions::private()).unwrap();

    assert_eq!(support::unix::mode_of(&file), 0o600);
    assert_eq!(fs::read_to_string(&file).unwrap(), "token");
}

/// GIVEN an exact mode
/// WHEN a new file is written
/// THEN it has exactly that mode regardless of the umask
#[cfg(unix)]
#[test]
fn exact_mode_is_applied_to_new_file() {
    let scratch = Scratch::new();
    let file = scratch.path("hook");
    let options = WriteOptions { mode: Mode::Exact(0o755), ..WriteOptions::default() };

    write_with(&file, "#!/bin/sh\n", &options).unwrap();

    assert_eq!(support::unix::mode_of(&file), 0o755);
}

/// GIVEN an existing file with mode 0640 and default options
/// WHEN it is rewritten
/// THEN the mode is still 0640
#[cfg(unix)]
#[test]
fn write_keeps_mode_of_existing_file() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "old");
    support::unix::set_mode(&file, 0o640);

    write(&file, "new").unwrap();

    assert_eq!(support::unix::mode_of(&file), 0o640);
}

/// GIVEN a file with nlink greater than one
/// WHEN it is rewritten with default options
/// THEN it is overwritten in place and every link sees the new content
#[cfg(unix)]
#[test]
fn write_overwrites_hardlinked_file_in_place() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "old contents");
    let other = scratch.path("b.txt");
    fs::hard_link(&file, &other).unwrap();
    let inode = support::unix::inode_of(&file);

    write(&file, "new").unwrap();

    assert_eq!(fs::read_to_string(&other).unwrap(), "new");
    assert_eq!(support::unix::inode_of(&file), inode);
    assert_eq!(support::unix::link_count(&file), 2);
}

/// GIVEN a file with nlink greater than one
/// WHEN it is rewritten with hardlink breaking
/// THEN the path gets a new inode and the other links keep the old content
#[cfg(unix)]
#[test]
fn atomic_write_can_break_hardlinks() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "old");
    let other = scratch.path("b.txt");
    fs::hard_link(&file, &other).unwrap();
    let options = WriteOptions { break_hardlinks: true, ..WriteOptions::default() };

    write_with(&file, "new", &options).unwrap();

    assert_eq!(fs::read_to_string(&file).unwrap(), "new");
    assert_eq!(fs::read_to_string(&other).unwrap(), "old");
    assert_eq!(support::unix::link_count(&file), 1);
}

/// GIVEN a hardlinked file and the private options
/// WHEN it is overwritten in place
/// THEN the exact mode is applied to the shared inode
#[cfg(unix)]
#[test]
fn hardlinked_overwrite_applies_exact_mode() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "old");
    support::unix::set_mode(&file, 0o644);
    fs::hard_link(&file, scratch.path("b.txt")).unwrap();

    write_with(&file, "secret", &WriteOptions::private()).unwrap();

    assert_eq!(support::unix::mode_of(&scratch.path("b.txt")), 0o600);
}

/// GIVEN the in-place option
/// WHEN an existing file is written with shorter content
/// THEN the inode and mode are kept and the file is truncated
#[cfg(unix)]
#[test]
fn in_place_write_keeps_inode_and_mode() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "a much longer old content");
    support::unix::set_mode(&file, 0o640);
    let inode = support::unix::inode_of(&file);
    let options = WriteOptions { in_place: true, ..WriteOptions::default() };

    write_with(&file, "short", &options).unwrap();

    assert_eq!(fs::read_to_string(&file).unwrap(), "short");
    assert_eq!(support::unix::inode_of(&file), inode);
    assert_eq!(support::unix::mode_of(&file), 0o640);
}

/// GIVEN the in-place option and a missing file
/// WHEN it is written
/// THEN the file is created
#[test]
fn in_place_write_creates_missing_file() {
    let scratch = Scratch::new();
    let file = scratch.path("a.txt");
    let options = WriteOptions { in_place: true, ..WriteOptions::default() };

    write_with(&file, "created", &options).unwrap();

    assert_eq!(read(&file).unwrap(), b"created");
}

/// GIVEN durability requested
/// WHEN an atomic write runs
/// THEN the content is written and no temporary file remains
#[test]
fn durable_write_completes() {
    let scratch = Scratch::new();
    let file = scratch.path("a.txt");
    let options = WriteOptions { durable: true, ..WriteOptions::default() };

    write_with(&file, "kept", &options).unwrap();
    write_with(&file, "kept again", &options).unwrap();

    assert_eq!(read(&file).unwrap(), b"kept again");
    assert_eq!(scratch.names(""), ["a.txt"]);
}

fn age(path: &std::path::Path) -> SystemTime {
    let old = SystemTime::now() - Duration::from_secs(3600);
    let file = fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(old).unwrap();
    old
}

/// GIVEN a file whose content already matches
/// WHEN it is written with skip-unchanged
/// THEN the file is not rewritten and its mtime is unchanged
#[test]
fn skip_unchanged_leaves_identical_file_alone() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "same");
    let old = age(&file);
    let options = WriteOptions { skip_unchanged: true, ..WriteOptions::default() };

    write_with(&file, "same", &options).unwrap();

    assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), old);
}

/// GIVEN a file whose content differs, with the same or another length
/// WHEN it is written with skip-unchanged
/// THEN the file holds the new content
#[test]
fn skip_unchanged_writes_different_content() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "same");
    let options = WriteOptions { skip_unchanged: true, ..WriteOptions::default() };

    write_with(&file, "sane", &options).unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "sane");
    write_with(&file, "longer", &options).unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "longer");
}

/// GIVEN a file with the right content but a looser mode
/// WHEN it is written with skip-unchanged and an exact mode
/// THEN the mode is corrected
#[cfg(unix)]
#[test]
fn skip_unchanged_still_applies_exact_mode() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "same");
    support::unix::set_mode(&file, 0o644);
    let options = WriteOptions { skip_unchanged: true, ..WriteOptions::private() };

    write_with(&file, "same", &options).unwrap();

    assert_eq!(support::unix::mode_of(&file), 0o600);
}

/// GIVEN a file that cannot be read
/// WHEN it is written with skip-unchanged and same-length content
/// THEN the write still succeeds
#[cfg(unix)]
#[test]
fn skip_unchanged_treats_unreadable_file_as_changed() {
    if support::unix::is_root() {
        return;
    }
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "same");
    support::unix::set_mode(&file, 0o200);
    let options = WriteOptions { skip_unchanged: true, ..WriteOptions::default() };

    write_with(&file, "sane", &options).unwrap();

    support::unix::set_mode(&file, 0o600);
    assert_eq!(fs::read_to_string(&file).unwrap(), "sane");
}

/// GIVEN a directory without write permission
/// WHEN a file is written in it
/// THEN a permission error naming the path is returned and no temp file remains
#[cfg(unix)]
#[test]
fn write_into_read_only_directory_fails_cleanly() {
    if support::unix::is_root() {
        return;
    }
    let scratch = Scratch::new();
    let dir = scratch.dir("locked");
    support::unix::set_mode(&dir, 0o500);
    let file = dir.join("a.txt");

    let err = write(&file, "x").unwrap_err();

    support::unix::set_mode(&dir, 0o700);
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    assert_eq!(err.path(), file);
    assert!(scratch.names("locked").is_empty());
}

static SERIAL: Mutex<()> = Mutex::new(());

/// GIVEN a relative path in the current directory
/// WHEN it is written
/// THEN the file is created beside the working directory
#[test]
fn write_accepts_bare_file_name() {
    let _serial = SERIAL.lock().unwrap_or_else(|err| err.into_inner());
    let scratch = Scratch::new();
    let previous = std::env::current_dir().unwrap();
    std::env::set_current_dir(scratch.root()).unwrap();

    let written = write("bare.txt", "x");

    std::env::set_current_dir(previous).unwrap();
    written.unwrap();
    assert_eq!(fs::read_to_string(scratch.path("bare.txt")).unwrap(), "x");
}
