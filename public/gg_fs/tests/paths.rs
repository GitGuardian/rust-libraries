mod support;

use std::path::{Path, PathBuf};

use gg_fs::paths::{
    CaseSensitivity, canonicalize, case_sensitivity, is_within, is_within_with, join,
    lexical_clean, project_relative, stable_hash,
};
use gg_fs::{Cause, Error};
use support::Scratch;

/// GIVEN a path that canonicalizes to the extended-length form on Windows
/// WHEN it is canonicalized
/// THEN the plain drive or UNC form is returned
#[cfg(windows)]
#[test]
fn canonicalize_strips_windows_verbatim_prefix() {
    let scratch = Scratch::new();
    let file = scratch.write("a.txt", "x");

    let canonical = canonicalize(&file).unwrap();

    assert!(!canonical.to_string_lossy().starts_with(r"\\?\"));
    assert!(canonical.is_file());
}

/// GIVEN a path with symlinks and dotdot segments
/// WHEN it is canonicalized
/// THEN the absolute resolved path is returned
#[cfg(unix)]
#[test]
fn canonicalize_resolves_symlinks_and_dotdot() {
    let scratch = Scratch::new();
    let target = scratch.write("real/file.txt", "x");
    support::unix::link(&scratch.path("real"), &scratch.path("alias"));

    let direct = canonicalize(scratch.path("alias/file.txt")).unwrap();
    let with_dotdot = canonicalize(scratch.path("alias/../alias/./file.txt")).unwrap();

    assert_eq!(direct, target);
    assert_eq!(with_dotdot, target);
}

/// GIVEN a path that does not exist
/// WHEN it is canonicalized
/// THEN a not-found error naming the path is returned
#[test]
fn canonicalize_reports_missing_path() {
    let scratch = Scratch::new();
    let missing = scratch.path("missing/file");

    let err = canonicalize(&missing).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert_eq!(err.path(), missing);
}

/// GIVEN paths with dot, dotdot, empty and trailing segments
/// WHEN each is cleaned
/// THEN each result is the normalized path
#[cfg(unix)]
#[test]
fn lexical_clean_normalizes_dot_dotdot_and_empty_segments() {
    let cases = [
        ("a/b/../c", "a/c"),
        ("./a", "a"),
        ("a//b", "a/b"),
        ("a/./b/.", "a/b"),
        ("/../a", "/a"),
        ("../a", "../a"),
        ("a/..", "."),
        ("", "."),
        ("/", "/"),
        ("a/../..", ".."),
        ("../../a/b", "../../a/b"),
        ("/a/b/../../..", "/"),
        ("a/b/", "a/b"),
        ("a/./../b", "b"),
        ("/a/./b", "/a/b"),
        ("a/b/c/../../d", "a/d"),
        (".", "."),
        ("./", "."),
        ("a/../b/../c", "c"),
        ("/a/b/../../../c", "/c"),
        ("..", ".."),
        ("../..", "../.."),
    ];

    for (input, expected) in cases {
        assert_eq!(lexical_clean(Path::new(input)), Path::new(expected), "{input:?}");
    }
}

/// GIVEN a path that is already clean
/// WHEN it is cleaned
/// THEN the input is borrowed, not copied
#[test]
fn lexical_clean_borrows_a_clean_path() {
    let clean = Path::new("a").join("b").join("c");

    assert!(matches!(lexical_clean(&clean), std::borrow::Cow::Borrowed(_)));
    assert!(matches!(lexical_clean(Path::new("a/../b")), std::borrow::Cow::Owned(_)));
}

/// GIVEN paths that do not exist
/// WHEN they are cleaned
/// THEN the result is computed without touching the filesystem
#[test]
fn lexical_clean_does_not_touch_the_filesystem() {
    let ghost = Path::new("/definitely/not/there/../here/./file");

    assert_eq!(lexical_clean(ghost), Path::new("/definitely/not/here/file"));
}

/// GIVEN /home/alice and /home/alicia/x
/// WHEN containment is checked
/// THEN the second is not inside the first
#[test]
fn is_within_compares_by_component() {
    assert!(is_within("/home/alice", "/home/alice"));
    assert!(is_within("/home/alice", "/home/alice/docs/a.txt"));
    assert!(!is_within("/home/alice", "/home/alicia/x"));
    assert!(!is_within("/home/alice/docs", "/home/alice"));
}

/// GIVEN a path that climbs out of the root with dotdot
/// WHEN containment is checked
/// THEN it is outside
#[test]
fn is_within_cannot_be_escaped_with_dotdot() {
    assert!(!is_within("/srv/app", "/srv/app/../secret"));
    assert!(is_within("/srv/app", "/srv/app/sub/../file"));
    assert!(!is_within("/srv/app", "/srv/appendix"));
}

/// GIVEN paths differing only by case
/// WHEN containment is checked under each policy
/// THEN they match only when insensitive
#[test]
fn is_within_honours_case_policy() {
    assert!(is_within_with("/Users/Alice", "/users/alice/Docs", CaseSensitivity::Insensitive));
    assert!(!is_within_with("/Users/Alice", "/users/alice/Docs", CaseSensitivity::Sensitive));
}

/// GIVEN a symlinked root
/// WHEN containment is checked
/// THEN the lexical forms differ until both sides are canonicalized
#[cfg(unix)]
#[test]
fn is_within_does_not_resolve_symlinks() {
    let scratch = Scratch::new();
    let real = scratch.dir("real");
    let file = scratch.write("real/a.txt", "x");
    let alias = scratch.path("alias");
    support::unix::link(&real, &alias);

    assert!(!is_within(&alias, &file));
    assert!(is_within(canonicalize(&alias).unwrap(), &file));
}

/// GIVEN a path inside the project
/// WHEN it is made project-relative
/// THEN the relative path with forward slashes is returned
#[test]
fn project_relative_returns_posix_path() {
    let project = Path::new("/work/app");

    assert_eq!(project_relative(Path::new("/work/app/src/main.rs"), project), "src/main.rs");
    assert_eq!(project_relative(Path::new("/work/app/./src/../lib.rs"), project), "lib.rs");
    assert_eq!(project_relative(Path::new("/work/app"), project), ".");
}

/// GIVEN a path outside the project
/// WHEN it is made project-relative
/// THEN the absolute path is returned
#[test]
fn project_relative_falls_back_for_outside_paths() {
    let project = Path::new("/work/app");

    assert_eq!(project_relative(Path::new("/work/other/lib.rs"), project), "/work/other/lib.rs");
    assert_eq!(project_relative(Path::new("/work/application/x"), project), "/work/application/x");
    assert_eq!(project_relative(Path::new("/work/app/../secret"), project), "/work/secret");
}

/// GIVEN a Windows path inside the project
/// WHEN it is made project-relative
/// THEN the result uses forward slashes
#[cfg(windows)]
#[test]
fn project_relative_converts_windows_separators() {
    let project = Path::new(r"C:\work\app");

    assert_eq!(project_relative(Path::new(r"C:\work\app\src\main.rs"), project), "src/main.rs");
    assert_eq!(project_relative(Path::new(r"c:\WORK\app\src\main.rs"), project), "src/main.rs");
    assert_eq!(project_relative(Path::new(r"D:\other\a.rs"), project), "D:/other/a.rs");
}

fn invalid(result: Result<PathBuf, Error>) -> String {
    match result.unwrap_err().cause() {
        Cause::InvalidPath(reason) => (*reason).to_owned(),
        other => panic!("unexpected cause {other}"),
    }
}

/// GIVEN an untrusted relative path starting with a slash
/// WHEN it is joined to a base
/// THEN an error is returned instead of replacing the base
#[test]
fn join_rejects_absolute_component() {
    let scratch = Scratch::new();

    for relative in ["/etc/passwd", "\\windows\\system32", "C:\\temp", "c:/temp", "C:"] {
        assert_eq!(invalid(join(scratch.root(), relative)), "is absolute", "{relative}");
    }
}

/// GIVEN a relative path with a dotdot segment
/// WHEN it is joined to a base
/// THEN an error is returned
#[test]
fn join_rejects_parent_traversal() {
    let scratch = Scratch::new();

    for relative in ["..", "a/../b", "../x", "a\\..\\b"] {
        assert_eq!(invalid(join(scratch.root(), relative)), "contains `..`", "{relative}");
    }
}

/// GIVEN a base containing a symlink to outside
/// WHEN a relative path through it is joined
/// THEN an error is returned
#[cfg(unix)]
#[test]
fn join_rejects_symlink_escape() {
    let scratch = Scratch::new();
    let base = scratch.dir("base");
    let outside = scratch.dir("outside");
    support::unix::link(&outside, &base.join("escape"));
    scratch.dir("base/inside");
    support::unix::link(&base.join("inside"), &base.join("fine"));

    assert_eq!(invalid(join(&base, "escape/file.txt")), "leaves the base through a symbolic link");
    assert_eq!(join(&base, "fine/file.txt").unwrap(), base.join("fine/file.txt"));
}

/// GIVEN a dangling symlink that leads outside the base
/// WHEN a path through it is joined
/// THEN an error is returned
#[cfg(unix)]
#[test]
fn join_rejects_dangling_symlink() {
    let scratch = Scratch::new();
    let base = scratch.dir("base");
    support::unix::link(&scratch.path("nowhere"), &base.join("dangling"));

    assert_eq!(invalid(join(&base, "dangling/file")), "leaves the base through a symbolic link");
}

/// GIVEN relative paths with empty and dot segments and both separators
/// WHEN they are joined
/// THEN the segments are dropped
#[test]
fn join_accepts_nested_paths_and_normalizes_separators() {
    let scratch = Scratch::new();
    let base = scratch.root();

    assert_eq!(join(base, "a/b.txt").unwrap(), base.join("a").join("b.txt"));
    assert_eq!(join(base, "a\\b.txt").unwrap(), base.join("a").join("b.txt"));
    assert_eq!(join(base, "./a//b/").unwrap(), base.join("a").join("b"));
    assert_eq!(join(base, "").unwrap(), base);
}

/// GIVEN a base that does not exist
/// WHEN a path is joined to it
/// THEN an I/O error is returned
#[test]
fn join_requires_an_existing_base() {
    let scratch = Scratch::new();

    let err = join(scratch.path("missing"), "a").unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

/// GIVEN names over 255 and paths over 4096 characters
/// WHEN they are validated
/// THEN they are rejected for both separators
#[test]
fn path_length_guard_checks_name_and_path_limits() {
    let scratch = Scratch::new();
    let long_name = "n".repeat(256);
    let ok_name = "n".repeat(255);
    let long_path = vec!["d"; 2100].join("/");

    assert_eq!(invalid(join(scratch.root(), &long_name)), "name is too long");
    assert_eq!(invalid(join(scratch.root(), &format!("a\\{long_name}"))), "name is too long");
    assert!(join(scratch.root(), &ok_name).is_ok());
    assert_eq!(invalid(join(scratch.root(), &long_path)), "path is too long");
}

/// GIVEN a name with a control character
/// WHEN it is joined
/// THEN it is rejected
#[test]
fn join_rejects_control_characters() {
    let scratch = Scratch::new();

    assert_eq!(invalid(join(scratch.root(), "a\nb")), "contains a control character");
    assert_eq!(invalid(join(scratch.root(), "tab\there")), "contains a control character");
}

/// GIVEN names that Windows does not allow
/// WHEN they are joined
/// THEN they are rejected
#[cfg(windows)]
#[test]
fn join_rejects_invalid_windows_names() {
    let scratch = Scratch::new();

    for name in ["CON", "nul.txt", "COM1", "a:b", "a?", "trailing.", "trailing "] {
        assert_eq!(
            invalid(join(scratch.root(), name)),
            "is not a valid Windows file name",
            "{name}"
        );
    }
}

/// GIVEN a path
/// WHEN its hash is computed
/// THEN it equals a pinned value
#[test]
fn stable_path_hash_is_stable_across_runs() {
    assert_eq!(stable_hash("/a/b"), 0xfc1a_90e6_c44e_a148);
    assert_eq!(stable_hash("a/b"), 0x8fb2_0ef6_3ced_4145);
    assert_eq!(stable_hash(""), 0xe3b0_c442_98fc_1c14);
    assert_eq!(stable_hash("/"), 0xade0_274f_84b8_28b0);
}

/// GIVEN spellings with the same components
/// WHEN they are hashed
/// THEN they hash equally, and different paths do not
#[test]
fn stable_path_hash_depends_on_components_only() {
    assert_eq!(stable_hash("a//b"), stable_hash("a/b"));
    assert_eq!(stable_hash("a/./b"), stable_hash("a/b"));
    assert_eq!(stable_hash(Path::new("a/b/")), stable_hash("a/b"));
    assert_ne!(stable_hash("a/b"), stable_hash("a/c"));
    assert_ne!(stable_hash("/a/b"), stable_hash("a/b"));
    assert_ne!(stable_hash("ab"), stable_hash("a/b"));
}

/// GIVEN a Windows path with backslashes
/// WHEN it is hashed
/// THEN it equals the hash of the forward slash spelling
#[cfg(windows)]
#[test]
fn stable_path_hash_treats_backslash_as_separator_on_windows() {
    assert_eq!(stable_hash(r"a\b"), stable_hash("a/b"));
}

/// GIVEN a directory on the host file system
/// WHEN its case sensitivity is queried
/// THEN it agrees with what the file system does for names differing in case
#[test]
fn case_sensitivity_matches_the_file_system() {
    let scratch = Scratch::new();
    scratch.write("Probe.txt", "x");
    let insensitive = scratch.path("probe.txt").exists();

    let reported = case_sensitivity(scratch.root()).unwrap();
    let default = CaseSensitivity::default();

    let expected =
        if insensitive { CaseSensitivity::Insensitive } else { CaseSensitivity::Sensitive };
    assert_eq!(reported, expected);
    assert_eq!(case_sensitivity(scratch.path("Probe.txt")).unwrap(), expected);
    if cfg!(windows) {
        assert_eq!(default, CaseSensitivity::Insensitive);
    } else {
        assert_eq!(default, CaseSensitivity::Sensitive);
    }
}

/// GIVEN a missing path
/// WHEN its case sensitivity is queried
/// THEN a not-found error is returned
#[test]
fn case_sensitivity_of_missing_path_is_an_error() {
    let scratch = Scratch::new();

    let err = case_sensitivity(scratch.path("missing")).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}
