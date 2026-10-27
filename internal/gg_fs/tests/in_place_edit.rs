mod support;

use std::fs;
use std::time::Duration;

use gg_fs::{Cause, Edit, FileLock, Replacement};
use sha2::{Digest, Sha256};
use support::Scratch;

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn replace(range: std::ops::Range<u64>, bytes: &str) -> Replacement {
    Replacement { range, bytes: bytes.as_bytes().to_vec() }
}

fn edit(original: &str, replacements: Vec<Replacement>) -> Edit {
    Edit::new(sha256(original.as_bytes()), replacements)
}

/// GIVEN a file whose hash matches the edit
/// WHEN the edit is applied
/// THEN the new bytes are written and the call reports success
#[test]
fn edit_applies_when_hash_matches() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");

    edit("hello world", vec![replace(6..11, "there")]).apply(&file).unwrap();

    assert_eq!(fs::read_to_string(&file).unwrap(), "hello there");
}

/// GIVEN several unordered replacements
/// WHEN the edit is applied
/// THEN all of them are made against the original offsets
#[test]
fn edit_applies_several_replacements_by_original_offset() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "one two three");

    let replacements = vec![replace(8..13, "3"), replace(0..3, "1"), replace(4..7, "2")];
    edit("one two three", replacements).apply(&file).unwrap();

    assert_eq!(fs::read_to_string(&file).unwrap(), "1 2 3");
}

/// GIVEN a file modified after the edit was computed
/// WHEN the edit is applied
/// THEN it is refused and the file is untouched
#[test]
fn edit_refuses_when_content_changed_since_computed() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "changed behind our back");

    let err = edit("hello world", vec![replace(0..5, "bye")]).apply(&file).unwrap_err();

    assert!(matches!(err.cause(), Cause::Changed));
    assert_eq!(fs::read_to_string(&file).unwrap(), "changed behind our back");
}

/// GIVEN overlapping replacements or one beyond the end of the file
/// WHEN the edit is applied
/// THEN an invalid-input error is returned and the file is untouched
#[test]
fn edit_refuses_overlapping_or_out_of_range_replacements() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");

    let overlapping = vec![replace(0..6, "x"), replace(4..8, "y")];
    let err = edit("hello world", overlapping).apply(&file).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

    let beyond = vec![replace(6..40, "x")];
    let err = edit("hello world", beyond).apply(&file).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

    assert_eq!(fs::read_to_string(&file).unwrap(), "hello world");
}

/// GIVEN a symlink to a file
/// WHEN the edit is applied
/// THEN it is refused without touching the target
#[cfg(unix)]
#[test]
fn edit_refuses_symlink() {
    let scratch = Scratch::new();
    let target = scratch.write("target.txt", "hello world");
    let link = scratch.path("link.txt");
    support::unix::link(&target, &link);

    let err = edit("hello world", vec![replace(0..5, "bye")]).apply(&link).unwrap_err();

    assert!(matches!(err.cause(), Cause::NotRegularFile));
    assert_eq!(fs::read_to_string(&target).unwrap(), "hello world");
}

/// GIVEN a directory or FIFO
/// WHEN the edit is applied
/// THEN it is refused
#[cfg(unix)]
#[test]
fn edit_refuses_non_regular_file() {
    let scratch = Scratch::new();
    let dir = scratch.dir("sub");
    let fifo = scratch.path("pipe");
    support::unix::make_fifo(&fifo);

    assert!(edit("", vec![]).apply(&dir).is_err());
    let err = edit("", vec![]).apply(&fifo).unwrap_err();
    assert!(matches!(err.cause(), Cause::NotRegularFile));
}

/// GIVEN a file with nlink one
/// WHEN the edit is applied
/// THEN the inode is unchanged and the mode is preserved
#[cfg(unix)]
#[test]
fn edit_with_single_link_writes_in_place() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    support::unix::set_mode(&file, 0o640);
    let inode = support::unix::inode_of(&file);

    edit("hello world", vec![replace(0..5, "bye")]).apply(&file).unwrap();

    assert_eq!(support::unix::inode_of(&file), inode);
    assert_eq!(support::unix::mode_of(&file), 0o640);
    assert_eq!(fs::read_to_string(&file).unwrap(), "bye world");
}

/// GIVEN a file with nlink greater than one
/// WHEN the edit is applied
/// THEN the file is replaced with the mode preserved and a single link
#[cfg(unix)]
#[test]
fn edit_with_multiple_links_replaces_atomically() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    support::unix::set_mode(&file, 0o640);
    let other = scratch.path("b.txt");
    fs::hard_link(&file, &other).unwrap();
    let inode = support::unix::inode_of(&file);

    edit("hello world", vec![replace(0..5, "bye")]).apply(&file).unwrap();

    assert_ne!(support::unix::inode_of(&file), inode);
    assert_eq!(support::unix::link_count(&file), 1);
    assert_eq!(support::unix::mode_of(&file), 0o640);
    assert_eq!(fs::read_to_string(&file).unwrap(), "bye world");
    assert_eq!(fs::read_to_string(&other).unwrap(), "hello world");
}

/// GIVEN crash-safe mode
/// WHEN the edit is applied
/// THEN the file is replaced rather than modified in place and the mode is kept
#[cfg(unix)]
#[test]
fn edit_crash_safe_mode_replaces_the_file() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    support::unix::set_mode(&file, 0o600);
    let inode = support::unix::inode_of(&file);
    let mut edit = edit("hello world", vec![replace(0..5, "bye")]);
    edit.crash_safe = true;

    edit.apply(&file).unwrap();

    assert_ne!(support::unix::inode_of(&file), inode);
    assert_eq!(support::unix::mode_of(&file), 0o600);
    assert_eq!(fs::read_to_string(&file).unwrap(), "bye world");
    assert!(!scratch.names("").iter().any(|name| name.starts_with(".gg_fs-")));
}

/// GIVEN backup requested
/// WHEN the edit is applied
/// THEN the original content is kept beside the file
#[test]
fn edit_can_keep_a_backup() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    let mut edit = edit("hello world", vec![replace(0..5, "bye")]);
    edit.backup = true;

    edit.apply(&file).unwrap();

    assert_eq!(fs::read_to_string(scratch.path("a.txt.bak")).unwrap(), "hello world");
    assert_eq!(fs::read_to_string(&file).unwrap(), "bye world");
}

/// GIVEN the paranoid constructor
/// WHEN the edit is applied
/// THEN the file is replaced and a backup is kept
#[test]
fn paranoid_edit_replaces_and_keeps_a_backup() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    let paranoid = Edit::paranoid(sha256(b"hello world"), vec![replace(0..5, "bye")]);

    paranoid.apply(&file).unwrap();

    assert_eq!(fs::read_to_string(&file).unwrap(), "bye world");
    assert_eq!(fs::read_to_string(scratch.path("a.txt.bak")).unwrap(), "hello world");
}

/// GIVEN a CRLF file with a BOM
/// WHEN bytes are replaced
/// THEN line endings and BOM outside the edited range are unchanged
#[test]
fn edit_preserves_line_endings_and_bom() {
    let scratch = Scratch::new();
    let original = "\u{feff}key=old\r\nother=1\r\n";
    let file = scratch.write("a.txt", original);
    let start = original.find("old").unwrap() as u64;

    edit(original, vec![replace(start..start + 3, "new")]).apply(&file).unwrap();

    assert_eq!(fs::read_to_string(&file).unwrap(), "\u{feff}key=new\r\nother=1\r\n");
}

/// GIVEN an applied edit
/// WHEN the directory is listed
/// THEN the lock file is left beside the file
#[test]
fn edit_leaves_its_lock_file_behind() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");

    edit("hello world", vec![replace(0..5, "bye")]).apply(&file).unwrap();

    assert!(scratch.path("a.txt.lock").exists());
}

/// GIVEN another process holding the lock of the file
/// WHEN the edit is applied
/// THEN a timeout error is returned and the file is untouched
#[test]
fn edit_is_refused_while_the_file_is_locked() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    let _held = FileLock::lock(scratch.path("a.txt.lock"), Duration::ZERO).unwrap();

    let err = edit("hello world", vec![replace(0..5, "bye")]).apply(&file).unwrap_err();

    assert!(matches!(err.cause(), Cause::Timeout(_)));
    assert_eq!(fs::read_to_string(&file).unwrap(), "hello world");
}
