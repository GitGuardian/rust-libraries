#![cfg(unix)]

mod support;

use std::fs;
use std::sync::OnceLock;

use gg_fs::{
    Cause, DirOptions, Mode, WriteOptions, create_dir_all_with, set_umask, write, write_with,
};
use support::Scratch;
use support::unix::{inode_of, link, mode_of, set_mode};

const HARDENED: u32 = 0o077;

fn host_umask() -> u32 {
    static HOST: OnceLock<u32> = OnceLock::new();
    *HOST.get_or_init(|| set_umask(HARDENED))
}

/// GIVEN a process that installed a restrictive umask
/// WHEN a file is written with default options
/// THEN it is created with the restrictive umask
#[test]
fn default_write_uses_the_process_umask() {
    host_umask();
    let scratch = Scratch::new();
    let file = scratch.path("state.json");

    write(&file, "{}").unwrap();

    assert_eq!(mode_of(&file), 0o666 & !HARDENED);
}

/// GIVEN a restrictive process umask and a new output file
/// WHEN a deliverable is written
/// THEN its mode is widened to the host umask
#[test]
fn output_write_honours_host_umask() {
    let host = host_umask();
    let scratch = Scratch::new();
    let file = scratch.path("report.json");

    write_with(&file, "{}", &WriteOptions::output()).unwrap();

    assert_eq!(mode_of(&file), 0o666 & !host);
}

/// GIVEN an existing output file with its own mode
/// WHEN a deliverable is written
/// THEN the content is replaced in place with the inode and mode unchanged
#[test]
fn output_write_keeps_inode_and_mode_of_existing_file() {
    host_umask();
    let scratch = Scratch::new();
    let file = scratch.write("report.json", "a much longer previous report");
    set_mode(&file, 0o600);
    let inode = inode_of(&file);

    write_with(&file, "{}", &WriteOptions::output()).unwrap();

    assert_eq!(fs::read_to_string(&file).unwrap(), "{}");
    assert_eq!(inode_of(&file), inode);
    assert_eq!(mode_of(&file), 0o600);
}

/// GIVEN a dangling symlink as the output path
/// WHEN a deliverable is written
/// THEN the target is created with the host umask mode and the link is kept
#[test]
fn output_write_follows_dangling_symlink_target() {
    let host = host_umask();
    let scratch = Scratch::new();
    let target = scratch.path("created.json");
    let path = scratch.path("report.json");
    link(&target, &path);

    write_with(&path, "{}", &WriteOptions::output()).unwrap();

    assert_eq!(fs::read_to_string(&target).unwrap(), "{}");
    assert_eq!(mode_of(&target), 0o666 & !host);
    assert!(fs::symlink_metadata(&path).unwrap().is_symlink());
}

/// GIVEN a special file as the output path
/// WHEN a deliverable is written
/// THEN a not-regular-file error is returned
#[test]
fn output_write_refuses_special_files() {
    host_umask();

    let err = write_with("/dev/null", "{}", &WriteOptions::output()).unwrap_err();

    assert!(matches!(err.cause(), Cause::NotRegularFile));
}

/// GIVEN a restrictive process umask
/// WHEN a directory is created with the host umask mode
/// THEN it gets the host umask mode
#[test]
fn host_umask_mode_applies_to_directories() {
    let host = host_umask();
    let scratch = Scratch::new();
    let dir = scratch.path("out/nested");
    let options = DirOptions { mode: Mode::HostUmask };

    create_dir_all_with(&dir, &options).unwrap();

    assert_eq!(mode_of(&dir), 0o777 & !host);
    assert_eq!(mode_of(&scratch.path("out")), 0o777 & !host);
}

/// GIVEN an installed umask
/// WHEN the umask is set again
/// THEN the installed value is returned
#[test]
fn set_umask_returns_the_previous_umask() {
    host_umask();

    assert_eq!(set_umask(HARDENED), HARDENED);
}
