mod support;

use std::fs;
use std::io::Read;
use std::time::{Duration, UNIX_EPOCH};

use gg_fs::{
    Cause, DEFAULT_READ_CAP, File, MetadataExt, OpenOptions, ReadOptions, Trust, is_binary,
    is_binary_bytes, metadata, read, read_to_string, read_with,
};
#[cfg(unix)]
use gg_fs::{MetadataOptions, metadata_with};
use support::Scratch;

fn capped(cap: u64) -> ReadOptions {
    ReadOptions { cap, ..ReadOptions::default() }
}

/// GIVEN a file and an empty file
/// WHEN they are read
/// THEN all bytes are returned
#[test]
fn read_returns_the_whole_file() {
    let scratch = Scratch::new();
    let file = scratch.write("a.bin", [0u8, 1, 2, 255]);
    let empty = scratch.write("empty", "");

    assert_eq!(read(&file).unwrap(), [0u8, 1, 2, 255]);
    assert!(read(&empty).unwrap().is_empty());
}

/// GIVEN a file smaller than or equal to the cap
/// WHEN it is read with a cap
/// THEN all bytes are returned
#[test]
fn read_capped_returns_content_under_cap() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "0123456789");

    assert_eq!(read_with(&file, &capped(10)).unwrap(), b"0123456789");
    assert_eq!(read_with(&file, &capped(100)).unwrap(), b"0123456789");
}

/// GIVEN a file larger than the cap
/// WHEN it is read with a cap
/// THEN a typed too-large error naming the path is returned
#[test]
fn read_capped_rejects_file_over_cap() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "0123456789");

    let err = read_with(&file, &capped(9)).unwrap_err();

    assert!(matches!(err.cause(), Cause::TooLarge { size: 10, cap: 9 }));
    assert_eq!(err.path(), file);
    assert!(err.to_string().contains("10 bytes exceeds the 9 byte limit"));
}

/// GIVEN the default options
/// WHEN the cap is inspected
/// THEN it is 64 MiB
#[test]
fn default_read_cap_is_64_mib() {
    assert_eq!(DEFAULT_READ_CAP, 64 * 1024 * 1024);
    assert_eq!(ReadOptions::default().cap, DEFAULT_READ_CAP);
    assert_eq!(ReadOptions::default().trust, Trust::Unchecked);
}

/// GIVEN a file with invalid UTF-8
/// WHEN it is read as a string
/// THEN a typed decoding error naming the path is returned
#[test]
fn read_to_string_rejects_invalid_utf8() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", [b'o', b'k', 0xff, 0xfe]);

    let err = read_to_string(&file).unwrap_err();

    assert!(matches!(err.cause(), Cause::NotUtf8(_)));
    assert_eq!(err.path(), file);
}

/// GIVEN a YAML config saved with a UTF-8 BOM
/// WHEN it is read as a string
/// THEN the BOM is removed, while raw bytes keep it
#[test]
fn read_to_string_removes_a_leading_bom() {
    let scratch = Scratch::new();
    let file = scratch.write("config.yaml", "\u{feff}key: value\n");

    assert_eq!(read_to_string(&file).unwrap(), "key: value\n");
    assert_eq!(read(&file).unwrap(), "\u{feff}key: value\n".as_bytes());
}

/// GIVEN a missing file
/// WHEN it is read
/// THEN a not-found error naming the path is returned
#[test]
fn read_of_missing_file_reports_not_found() {
    let scratch = Scratch::new();
    let file = scratch.path("missing.txt");

    let err = read(&file).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert_eq!(err.path(), file);
}

/// GIVEN a missing file and a file without read permission
/// WHEN each is read
/// THEN the errors are distinguishable kinds
#[cfg(unix)]
#[test]
fn missing_file_is_distinct_from_unreadable_file() {
    if support::unix::is_root() {
        return;
    }
    let scratch = Scratch::new();
    let locked = scratch.write("locked.txt", "x");
    support::unix::set_mode(&locked, 0o000);

    let unreadable = read(&locked).unwrap_err();
    let missing = read(scratch.path("missing.txt")).unwrap_err();

    assert_eq!(unreadable.kind(), std::io::ErrorKind::PermissionDenied);
    assert_eq!(missing.kind(), std::io::ErrorKind::NotFound);
}

/// GIVEN a symlink to a file
/// WHEN it is read with a trust requirement
/// THEN the link is refused as a regular file
#[cfg(unix)]
#[test]
fn trusted_read_does_not_follow_symlinks() {
    let scratch = Scratch::new();
    let target = scratch.write("target.txt", "x");
    let link = scratch.path("link.txt");
    support::unix::link(&target, &link);

    let trusted = ReadOptions { trust: Trust::Caller, ..ReadOptions::default() };
    let err = read_with(&link, &trusted).unwrap_err();

    assert!(matches!(err.cause(), Cause::NotRegularFile));
    assert_eq!(read(&link).unwrap(), b"x");
}

/// GIVEN paths with known binary extensions in mixed case
/// WHEN they are classified
/// THEN each is binary without being opened
#[test]
fn binary_detection_by_extension() {
    for name in ["image.PNG", "archive.Tar.Gz", "lib.dll", "doc.pdf", "db.sqlite", "x.ZIP"] {
        assert!(is_binary(format!("/does/not/exist/{name}")).unwrap(), "{name}");
    }
}

/// GIVEN files with a text extension
/// WHEN they are classified by content
/// THEN NUL bytes make a file binary, except in UTF-16 or UTF-32 text
#[test]
fn binary_detection_by_content_sniff() {
    let scratch = Scratch::new();
    let text = scratch.write("plain.txt", "just text\n");
    let nul = scratch.write("blob.txt", b"abc\0def");
    let utf16 = scratch.write("wide.txt", [0xff, 0xfe, b'a', 0, b'b', 0]);
    let empty = scratch.write("empty.txt", "");

    assert!(!is_binary(&text).unwrap());
    assert!(is_binary(&nul).unwrap());
    assert!(!is_binary(&utf16).unwrap());
    assert!(!is_binary(&empty).unwrap());
}

/// GIVEN a NUL byte after the sniffed prefix
/// WHEN the content is classified
/// THEN it is not binary
#[test]
fn binary_sniff_examines_only_the_leading_bytes() {
    let mut head = vec![b'a'; 8 * 1024];
    head.push(0);

    assert!(!is_binary_bytes(&head));
    assert!(is_binary_bytes(b"a\0"));
    assert!(!is_binary_bytes(&[0x00, 0x00, 0xfe, 0xff, 0, 0]));
}

/// GIVEN a missing file with a text extension
/// WHEN it is classified
/// THEN the open error is returned
#[test]
fn binary_detection_reports_missing_file() {
    let scratch = Scratch::new();

    let err = is_binary(scratch.path("missing.txt")).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

/// GIVEN a file opened with the no-cache option
/// WHEN it is read
/// THEN the content is the same
#[test]
fn open_with_no_cache_reads_the_file() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "scanned");
    let options = OpenOptions { no_cache: true, ..OpenOptions::default() };

    let mut opened = File::open_with(&file, &options).unwrap();
    let mut content = String::new();
    opened.read_to_string(&mut content).unwrap();

    assert_eq!(content, "scanned");
    assert_eq!(opened.path(), file);
}

/// GIVEN a symlink to a file
/// WHEN its metadata is read without following
/// THEN the link itself is described
#[cfg(unix)]
#[test]
fn metadata_is_read_without_following_symlinks() {
    let scratch = Scratch::new();
    let target = scratch.write("target.txt", "twelve bytes");
    let link = scratch.path("link.txt");
    support::unix::link(&target, &link);

    let own = metadata_with(&link, &MetadataOptions { follow_symlinks: false }).unwrap();
    let followed = metadata(&link).unwrap();

    assert!(own.is_symlink());
    assert!(followed.is_file());
    assert_eq!(followed.len(), 12);
}

/// GIVEN a missing path
/// WHEN its metadata is read
/// THEN a not-found error naming the path is returned
#[test]
fn metadata_of_missing_path_names_the_path() {
    let scratch = Scratch::new();
    let path = scratch.path("missing");

    let err = metadata(&path).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert_eq!(err.path(), path);
}

/// GIVEN files with known modification times
/// WHEN the mtime is encoded
/// THEN it is nanoseconds since the Unix epoch, negative before it
#[test]
fn mtime_has_one_wire_representation() {
    let scratch = Scratch::new();
    let after = scratch.write("after", "x");
    let before = scratch.write("before", "x");
    fs::OpenOptions::new()
        .write(true)
        .open(&after)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_millis(1500))
        .unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&before)
        .unwrap()
        .set_modified(UNIX_EPOCH - Duration::from_secs(2))
        .unwrap();

    assert_eq!(metadata(&after).unwrap().modified_nanos(), Some(1_500_000_000));
    assert_eq!(metadata(&before).unwrap().modified_nanos(), Some(-2_000_000_000));
}
