mod support;

use std::error::Error as _;

#[cfg(unix)]
use gg_fs::UntrustedReason;
use gg_fs::{Cause, Error, ReadOptions, ResultExt, read, read_with};
use support::Scratch;

fn missing_file_error() -> (Scratch, std::path::PathBuf, Error) {
    let scratch = Scratch::new();
    let path = scratch.path("dir with spaces/missing file.txt");
    let err = read(&path).unwrap_err();
    (scratch, path, err)
}

/// GIVEN a failed open
/// WHEN the error is displayed
/// THEN it names the operation, the path and the io cause
#[test]
fn error_carries_operation_path_and_source() {
    let (_scratch, path, err) = missing_file_error();

    assert_eq!(err.op(), "open");
    assert_eq!(err.path(), path);
    assert!(matches!(err.cause(), Cause::Io(_)));
    let io_message = err.source().unwrap().to_string();
    assert_eq!(err.to_string(), format!("open {}: {io_message}", path.display()));
    assert!(io_message.contains("(os error"), "{io_message}");
}

/// GIVEN a missing file
/// WHEN the error kind is inspected
/// THEN NotFound is reported
#[test]
fn error_exposes_kind_for_matching() {
    let (_scratch, _path, err) = missing_file_error();

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

/// GIVEN a path with spaces
/// WHEN the error is displayed
/// THEN the path is shown unquoted
#[test]
fn error_display_uses_the_plain_path() {
    let (_scratch, path, err) = missing_file_error();

    let message = err.to_string();

    assert!(message.contains(&path.display().to_string()));
    assert!(!message.contains('"'));
    assert!(!message.contains('\''));
}

/// GIVEN an fs error
/// WHEN its source chain is walked
/// THEN the io error is the root
#[test]
fn error_source_chain_is_preserved() {
    let (_scratch, _path, err) = missing_file_error();

    let cause = err.source().unwrap();
    let root = cause.source().unwrap();

    assert!(cause.is::<Cause>());
    assert_eq!(root.downcast_ref::<std::io::Error>().unwrap().kind(), std::io::ErrorKind::NotFound);
    assert!(root.source().is_none());
}

/// GIVEN an fs error
/// WHEN it is converted into a boxed error
/// THEN the message and the downcast are kept
#[test]
fn error_converts_into_boxed_error() {
    let (_scratch, path, err) = missing_file_error();
    let expected = err.to_string();

    let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(err);

    assert_eq!(boxed.to_string(), expected);
    assert_eq!(boxed.downcast_ref::<Error>().unwrap().path(), path);
}

/// GIVEN an over-cap file
/// WHEN the error is displayed
/// THEN both numbers appear
#[test]
fn too_large_error_reports_size_and_cap() {
    let scratch = Scratch::new();
    let file = scratch.write("big", "0123456789");

    let err = read_with(&file, &ReadOptions { cap: 4, ..ReadOptions::default() }).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::FileTooLarge);
    assert!(err.to_string().ends_with("10 bytes exceeds the 4 byte limit"), "{err}");
}

/// GIVEN a file that is not valid UTF-8
/// WHEN the error is displayed
/// THEN it names the path and the problem
#[test]
fn not_utf8_error_names_path_and_problem() {
    let scratch = Scratch::new();
    let file = scratch.write("bin", [0xff, 0xfe]);

    let err = gg_fs::read_to_string(&file).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(err.to_string(), format!("read {}: not valid UTF-8", file.display()));
}

/// GIVEN a file that fails a trust requirement
/// WHEN the error is displayed
/// THEN the reason is stated
#[cfg(unix)]
#[test]
fn untrusted_error_states_the_reason() {
    let scratch = Scratch::new();
    let file = scratch.write("loose", "x");
    support::unix::set_mode(&file, 0o666);
    let options = ReadOptions { trust: gg_fs::Trust::Caller, ..ReadOptions::default() };

    let err = read_with(&file, &options).unwrap_err();

    assert!(matches!(err.cause(), Cause::Untrusted(UntrustedReason::Writable)));
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(err.to_string().ends_with("not trusted: writable by group or others"), "{err}");
}

/// GIVEN a failing cache read
/// WHEN `ok_or_log` runs
/// THEN none is returned for the error and the value for a success
#[test]
fn ok_or_log_maps_error_to_none() {
    let scratch = Scratch::new();
    let file = scratch.write("present", "x");

    assert_eq!(read(scratch.path("absent")).ok_or_log(), None);
    assert_eq!(read(&file).ok_or_log(), Some(b"x".to_vec()));
}
