mod support;

use gg_fs::{Cause, Edit, Replacement};
use sha2::{Digest, Sha256};
use std::fs;
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

/// GIVEN a caller other than root and a file with nlink one
/// WHEN the edit is applied
/// THEN the inode is unchanged and the mode is preserved
#[cfg(unix)]
#[test]
fn edit_with_single_link_writes_in_place() {
    if support::unix::is_root() {
        return;
    }
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

/// GIVEN a file and a neighbor named like its lock file, such as `Gemfile` and
/// `Gemfile.lock`
/// WHEN the file is edited, in place and crash-safe
/// THEN the neighbor keeps its contents
#[test]
fn edit_leaves_a_neighboring_lock_file_alone() {
    let scratch = Scratch::new();
    let gemfile = scratch.write("Gemfile", "gem 'rails'");
    let lockfile = scratch.write("Gemfile.lock", "GEM\n  specs:\n");

    edit("gem 'rails'", vec![replace(5..10, "rack")]).apply(&gemfile).unwrap();
    let mut crash_safe = edit("gem 'rack'", vec![replace(5..9, "rails")]);
    crash_safe.crash_safe = true;
    crash_safe.apply(&gemfile).unwrap();

    assert_eq!(fs::read_to_string(&gemfile).unwrap(), "gem 'rails'");
    assert_eq!(fs::read_to_string(&lockfile).unwrap(), "GEM\n  specs:\n");
}

/// GIVEN a folder with one file
/// WHEN the file is edited, in place and crash-safe
/// THEN the folder holds the same names as before
#[test]
fn edit_creates_no_file_beside_its_target() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");

    edit("hello world", vec![replace(0..5, "bye")]).apply(&file).unwrap();
    let mut crash_safe = edit("bye world", vec![replace(0..3, "hi")]);
    crash_safe.crash_safe = true;
    crash_safe.apply(&file).unwrap();

    assert_eq!(scratch.names(""), ["a.txt"]);
    assert_eq!(fs::read_to_string(&file).unwrap(), "hi world");
}

/// GIVEN another edit holding the file
/// WHEN the edit is applied
/// THEN a timeout error is returned and the file is untouched
#[test]
fn edit_is_refused_while_another_edit_holds_the_file() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    let held = support::hold_lock(&file);

    let err = edit("hello world", vec![replace(0..5, "bye")]).apply(&file).unwrap_err();

    drop(held);
    assert!(matches!(err.cause(), Cause::Timeout(_)));
    assert_eq!(fs::read_to_string(&file).unwrap(), "hello world");
}

/// GIVEN an applied edit
/// WHEN another edit holds the file afterwards
/// THEN it gets the lock, because the first edit released it
#[test]
fn edit_releases_the_file_when_done() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");

    edit("hello world", vec![replace(0..5, "bye")]).apply(&file).unwrap();

    drop(support::hold_lock(&file));
}

/// GIVEN a backup path that is a hard link to another file
/// WHEN the edit is applied with a backup
/// THEN the backup is replaced and the other file keeps its contents
#[cfg(unix)]
#[test]
fn edit_backup_does_not_write_through_a_hard_link() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    let victim = scratch.write("victim", "keep me");
    fs::hard_link(&victim, scratch.path("a.txt.bak")).unwrap();
    let mut edit = edit("hello world", vec![replace(0..5, "bye")]);
    edit.backup = true;

    edit.apply(&file).unwrap();

    assert_eq!(fs::read_to_string(&victim).unwrap(), "keep me");
    assert_eq!(fs::read_to_string(scratch.path("a.txt.bak")).unwrap(), "hello world");
}

/// GIVEN a backup path that is a symbolic link to another file
/// WHEN the edit is applied with a backup
/// THEN the link is replaced by the backup and the other file keeps its contents
#[cfg(unix)]
#[test]
fn edit_backup_replaces_a_symbolic_link() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    let victim = scratch.write("victim", "keep me");
    support::unix::link(&victim, &scratch.path("a.txt.bak"));
    let mut edit = edit("hello world", vec![replace(0..5, "bye")]);
    edit.backup = true;

    edit.apply(&file).unwrap();

    assert_eq!(fs::read_to_string(&victim).unwrap(), "keep me");
    assert!(fs::symlink_metadata(scratch.path("a.txt.bak")).unwrap().is_file());
    assert_eq!(fs::read_to_string(scratch.path("a.txt.bak")).unwrap(), "hello world");
}

/// GIVEN a file whose group differs from the one a new file in its folder gets
/// WHEN it is edited crash-safe
/// THEN the replacement keeps the owner and group of the original
#[cfg(unix)]
#[test]
fn crash_safe_edit_keeps_owner_and_group() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    let (uid, default_gid) = support::unix::owner_of(&file);
    let Some(gid) = support::unix::other_group(default_gid) else { return };
    support::unix::chown(&file, uid, gid);
    let mut edit = edit("hello world", vec![replace(0..5, "bye")]);
    edit.crash_safe = true;
    edit.backup = true;

    edit.apply(&file).unwrap();

    assert_eq!(support::unix::owner_of(&file), (uid, gid));
    assert_eq!(support::unix::owner_of(&scratch.path("a.txt.bak")), (uid, gid));
}

/// GIVEN root and a file owned by another user, with one or several links
/// WHEN root edits it, in place and crash-safe
/// THEN the file is replaced and keeps its owner, group and mode
#[cfg(unix)]
#[test]
fn root_edit_replaces_the_file_and_keeps_its_owner() {
    if !support::unix::is_root() {
        return;
    }
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "hello world");
    support::unix::chown(&file, 1, 1);
    support::unix::set_mode(&file, 0o640);
    let inode = support::unix::inode_of(&file);

    edit("hello world", vec![replace(0..5, "bye")]).apply(&file).unwrap();
    assert_ne!(support::unix::inode_of(&file), inode);
    assert_eq!(support::unix::owner_of(&file), (1, 1));
    assert_eq!(support::unix::mode_of(&file), 0o640);

    let mut crash_safe = edit("bye world", vec![replace(0..3, "hi")]);
    crash_safe.crash_safe = true;
    crash_safe.backup = true;
    crash_safe.apply(&file).unwrap();
    assert_eq!(support::unix::owner_of(&file), (1, 1));
    assert_eq!(support::unix::owner_of(&scratch.path("a.txt.bak")), (1, 1));

    fs::hard_link(&file, scratch.path("b.txt")).unwrap();
    edit("hi world", vec![replace(0..2, "yo")]).apply(&file).unwrap();
    assert_eq!(support::unix::owner_of(&file), (1, 1));
    assert_eq!(fs::read_to_string(&file).unwrap(), "yo world");
    assert_eq!(fs::read_to_string(scratch.path("b.txt")).unwrap(), "hi world");
}

#[cfg(unix)]
const OTHER_USER_EDIT_ENV: &str = "GG_FS_TEST_OTHER_USER_EDIT";

/// Edits the file named by the environment with a backup and reports the
/// outcome. It does nothing when run as a regular test.
#[cfg(unix)]
#[test]
fn other_user_edit_process() {
    if let Ok(path) = std::env::var(OTHER_USER_EDIT_ENV) {
        let mut edit = edit("hello world", vec![replace(0..5, "bye")]);
        edit.backup = true;
        match edit.apply(&path) {
            Ok(()) => println!("edited"),
            Err(err) => println!("failed: {err}"),
        }
    }
}

/// GIVEN root, and a writable file owned by another user in a writable folder
/// WHEN a third user edits it in place with a backup
/// THEN the edit succeeds, the file keeps its owner, and the backup belongs to
/// the editor and only the editor can read it
#[cfg(unix)]
#[test]
fn backup_of_another_users_file_is_private_to_the_editor() {
    use std::os::unix::process::CommandExt;

    if !support::unix::is_root() {
        return;
    }
    let scratch = Scratch::new();
    support::unix::set_mode(scratch.root(), 0o777);
    let file = scratch.write("a.txt", "hello world");
    support::unix::chown(&file, 1, 1);
    support::unix::set_mode(&file, 0o666);

    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "other_user_edit_process", "--nocapture", "--test-threads=1"])
        .env(OTHER_USER_EDIT_ENV, &file)
        .uid(2)
        .gid(2)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.lines().any(|line| line.ends_with("edited")), "{stdout}");
    assert_eq!(support::unix::owner_of(&file), (1, 1));
    assert_eq!(fs::read_to_string(&file).unwrap(), "bye world");
    let backup = scratch.path("a.txt.bak");
    assert_eq!(support::unix::owner_of(&backup), (2, 2));
    assert_eq!(support::unix::mode_of(&backup), 0o600);
}

/// GIVEN a link to a folder that is switched back and forth to another folder,
/// where the file of the same name has other contents
/// WHEN files are edited crash-safe through the link many times
/// THEN no edit ever lands in the other folder
#[cfg(unix)]
#[test]
fn crash_safe_edit_never_lands_in_a_switched_in_folder() {
    use std::os::unix::fs::symlink;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    const FILES: usize = 300;
    let scratch = Scratch::new();
    let names: Vec<String> = (0..FILES).map(|i| format!("f{i}")).collect();
    for name in &names {
        scratch.write(&format!("real/{name}"), "hello world");
        scratch.write(&format!("outside/{name}"), "untouched");
    }
    let (real, outside, link) =
        (scratch.path("real"), scratch.path("outside"), scratch.path("link"));
    symlink(&real, &link).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let switcher = {
        let (real, outside, link, stop) =
            (real.clone(), outside.clone(), link.clone(), stop.clone());
        let next = scratch.path("next");
        thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                for target in [&outside, &real] {
                    symlink(target, &next).unwrap();
                    fs::rename(&next, &link).unwrap();
                }
            }
        })
    };

    let mut edited = 0;
    for name in &names {
        let mut edit = edit("hello world", vec![replace(0..5, "bye")]);
        edit.crash_safe = true;
        edited += usize::from(edit.apply(link.join(name)).is_ok());
    }
    stop.store(true, Ordering::Relaxed);
    switcher.join().unwrap();

    assert!(edited > 0);
    for name in &names {
        assert_eq!(fs::read_to_string(outside.join(name)).unwrap(), "untouched", "{name}");
    }
}
