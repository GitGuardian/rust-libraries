use gg_fs::glob::Patterns;

/// GIVEN patterns with wildcards, anchors and a trailing slash
/// WHEN paths are matched
/// THEN a star stays within a component and a double star spans components
#[test]
fn exclusion_globs_follow_the_documented_semantics() {
    let cases: &[(&str, &[&str], &[&str])] = &[
        ("*.py", &["a.py", "dir/a.py"], &[".py", "a.pyc", "dir.py/b"]),
        ("/a.py", &["a.py"], &["dir/a.py"]),
        ("a/b", &["a/b", "x/a/b"], &["a/bc", "a/x/b"]),
        ("a/*", &["a/b", "x/a/b"], &["a", "a/b/c/d/e"]),
        ("**/x", &["x", "a/x", "a/b/x"], &["xx", "a/xy"]),
        ("a/**/b", &["a/b", "a/x/b", "a/x/y/b"], &["ab", "x/b"]),
        ("docs/", &["docs/", "docs/x", "x/docs/"], &["docs", "mydocs/x"]),
        ("/docs/", &["docs/", "docs/x"], &["x/docs/"]),
        ("a.b", &["a.b"], &["aXb", "a+b"]),
        ("(x)", &["(x)"], &["x"]),
    ];

    for (pattern, matching, other) in cases {
        let patterns = Patterns::new([pattern]).unwrap();
        for path in *matching {
            assert!(patterns.matches(path), "pattern {pattern:?} should match {path:?}");
        }
        for path in *other {
            assert!(!patterns.matches(path), "pattern {pattern:?} should not match {path:?}");
        }
    }
}

/// GIVEN a pattern ending in a double star
/// WHEN paths below the directory are matched
/// THEN every depth matches
#[test]
fn trailing_double_star_matches_every_depth() {
    let patterns = Patterns::new(["a/**"]).unwrap();

    for path in ["a", "a/b", "a/b/c", "x/a/b/c/d"] {
        assert!(patterns.matches(path), "{path}");
    }
    assert!(!patterns.matches("ab/c"));
    assert!(Patterns::new(["**"]).unwrap().matches("any/path/at/all"));
}

/// GIVEN several patterns
/// WHEN a path is matched
/// THEN it matches if any pattern matches
#[test]
fn a_path_matches_the_set_if_any_pattern_matches() {
    let patterns = Patterns::new(["*.log", "build/"]).unwrap();

    assert!(patterns.matches("a.log"));
    assert!(patterns.matches("build/out"));
    assert!(!patterns.matches("src/main.rs"));
}

/// GIVEN no patterns
/// WHEN a path is matched
/// THEN nothing matches
#[test]
fn empty_pattern_set_matches_nothing() {
    let patterns = Patterns::new(Vec::<String>::new()).unwrap();

    assert!(!patterns.matches("anything"));
    assert!(!Patterns::default().matches("anything"));
}

/// GIVEN invalid patterns
/// WHEN they are compiled
/// THEN each is reported with its pattern and the offset of the problem
#[test]
fn exclusion_globs_reject_invalid_patterns() {
    let invalid = [
        ("", 0),
        ("***", 0),
        ("a**", 1),
        ("**a", 0),
        ("a/**b", 2),
        ("a//b", 1),
        ("/", 0),
        ("**/", 0),
    ];

    for (pattern, offset) in invalid {
        let err = Patterns::new([pattern]).unwrap_err();
        assert_eq!(err.pattern, pattern);
        assert_eq!(err.offset, offset, "{pattern:?}: {err}");
        assert!(err.to_string().contains("invalid pattern"), "{err}");
    }
}

/// GIVEN a valid pattern followed by an invalid one
/// WHEN they are compiled
/// THEN the invalid pattern is reported
#[test]
fn the_first_invalid_pattern_is_reported() {
    let err = Patterns::new(["*.py", "a**", "***"]).unwrap_err();

    assert_eq!(err.pattern, "a**");
}

/// GIVEN a Windows path
/// WHEN it is matched
/// THEN backslashes are converted before matching
#[test]
fn exclusion_matching_converts_backslashes() {
    let patterns = Patterns::new(["src/*.rs", "/build/"]).unwrap();

    assert!(patterns.matches("src\\main.rs"));
    assert!(patterns.matches("build\\out\\a.o"));
    assert!(!patterns.matches("lib\\main.rs"));
}

/// GIVEN a pattern with non-ASCII characters
/// WHEN paths are matched
/// THEN they are matched literally
#[test]
fn exclusion_globs_handle_non_ascii_paths() {
    let patterns = Patterns::new(["données/*.日本"]).unwrap();

    assert!(patterns.matches("données/a.日本"));
    assert!(!patterns.matches("donnees/a.日本"));
}
