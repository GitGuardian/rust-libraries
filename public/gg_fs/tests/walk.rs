mod support;

use std::path::Path;
use std::sync::Mutex;

use gg_fs::{Cause, DirEntry, WalkControl, WalkOptions, walk, walk_parallel, walk_with};
use support::Scratch;

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/")
}

fn files(root: &Path, options: &WalkOptions) -> Vec<String> {
    let mut found: Vec<String> = walk_with(root, options)
        .unwrap()
        .map(|entry| relative(root, &entry.unwrap().path))
        .collect();
    found.sort();
    found
}

fn visited(root: &Path, options: &WalkOptions) -> Vec<(String, bool)> {
    visited_with(root, options, |item| Some(item.unwrap()))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn visited_readable(root: &Path, options: &WalkOptions) -> Vec<(String, bool)> {
    visited_with(root, options, Result::ok)
}

fn visited_with<F>(root: &Path, options: &WalkOptions, keep: F) -> Vec<(String, bool)>
where
    F: Fn(Result<DirEntry, gg_fs::Error>) -> Option<DirEntry> + Sync,
{
    let seen = Mutex::new(Vec::new());
    walk_parallel(root, options, || {
        let seen = &seen;
        let keep = &keep;
        move |item| {
            if let Some(entry) = keep(item) {
                seen.lock().unwrap().push((relative(root, &entry.path), entry.file_type.is_dir()));
            }
            WalkControl::Continue
        }
    })
    .unwrap();
    let mut seen = seen.into_inner().unwrap();
    seen.sort();
    seen
}

fn tree() -> Scratch {
    let scratch = Scratch::new();
    scratch.write("a.txt", "a");
    scratch.write("sub/b.txt", "b");
    scratch.write("sub/deep/c.txt", "c");
    scratch
}

/// GIVEN a tree of files
/// WHEN it is walked
/// THEN every file is yielded once
#[test]
fn walk_visits_regular_files() {
    let scratch = tree();

    let mut found: Vec<String> = walk(scratch.root())
        .unwrap()
        .map(|entry| relative(scratch.root(), &entry.unwrap().path))
        .collect();
    found.sort();

    assert_eq!(found, ["a.txt", "sub/b.txt", "sub/deep/c.txt"]);
}

/// GIVEN a root that is a file
/// WHEN it is walked
/// THEN the file itself is yielded
#[test]
fn walk_of_a_file_root_yields_that_file() {
    let scratch = tree();
    let file = scratch.path("a.txt");

    let found: Vec<DirEntry> = walk(&file).unwrap().map(Result::unwrap).collect();

    let [entry] = found.as_slice() else { panic!("expected one entry, found {found:?}") };
    assert_eq!(entry.path, file);
    assert!(entry.file_type.is_file());
}

/// GIVEN a missing root
/// WHEN it is walked
/// THEN a not-found error is yielded
#[test]
fn walk_of_missing_root_yields_an_error() {
    let scratch = Scratch::new();
    let root = scratch.path("missing");

    let results: Vec<_> = walk(&root).unwrap().collect();

    let [result] = results.as_slice() else { panic!("expected one result") };
    assert_eq!(result.as_ref().unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

/// GIVEN hidden files and a .git directory
/// WHEN it is walked
/// THEN they are yielded
#[test]
fn walk_includes_hidden_files_and_git_directories() {
    let scratch = Scratch::new();
    scratch.write(".env", "x");
    scratch.write(".git/config", "x");
    scratch.write(".hidden/inner", "x");

    let found = files(scratch.root(), &WalkOptions::default());

    assert_eq!(found, [".env", ".git/config", ".hidden/inner"]);
}

/// GIVEN a tree with a gitignore and an ignore file
/// WHEN it is walked
/// THEN gitignored paths are skipped and ignore files have no effect
#[test]
fn walk_respects_gitignore() {
    let scratch = tree();
    scratch.write(".gitignore", "ignored.txt\nsub/deep/\n");
    scratch.write(".ignore", "a.txt\n");
    scratch.write("ignored.txt", "x");

    let found = files(scratch.root(), &WalkOptions::default());

    assert_eq!(found, [".gitignore", ".ignore", "a.txt", "sub/b.txt"]);
}

/// GIVEN gitignore handling turned off
/// WHEN the tree is walked
/// THEN ignored paths are yielded
#[test]
fn walk_can_ignore_gitignore() {
    let scratch = tree();
    scratch.write(".gitignore", "a.txt\n");
    let options = WalkOptions { respect_gitignore: false, ..WalkOptions::default() };

    assert!(files(scratch.root(), &options).contains(&"a.txt".to_owned()));
}

/// GIVEN a root below a directory with a gitignore
/// WHEN it is walked
/// THEN the gitignore of the parent applies
#[test]
fn walk_applies_gitignore_of_parent_directories() {
    let scratch = tree();
    scratch.write(".gitignore", "b.txt\n");

    let found = files(&scratch.path("sub"), &WalkOptions::default());

    assert_eq!(found, ["deep/c.txt"]);
}

/// GIVEN include and exclude overrides
/// WHEN it is walked
/// THEN overrides take precedence
#[test]
fn walk_applies_override_globs() {
    let scratch = tree();
    scratch.write(".gitignore", "b.txt\n");
    scratch.write("sub/d.rs", "d");

    let include =
        WalkOptions { overrides: vec!["*.rs".into(), "b.txt".into()], ..WalkOptions::default() };
    let exclude = WalkOptions { overrides: vec!["!sub/".into()], ..WalkOptions::default() };

    assert_eq!(files(scratch.root(), &include), ["sub/b.txt", "sub/d.rs"]);
    assert_eq!(files(scratch.root(), &exclude), [".gitignore", "a.txt"]);
}

/// GIVEN an invalid override glob
/// WHEN the walk is created
/// THEN a parse error is returned
#[test]
fn walk_rejects_invalid_override_glob() {
    let scratch = tree();
    let options = WalkOptions { overrides: vec!["[".into()], ..WalkOptions::default() };

    let err = walk_with(scratch.root(), &options).err().unwrap();

    assert!(matches!(err.cause(), Cause::Parse(_)));
    let parallel = walk_parallel(scratch.root(), &options, || |_| WalkControl::Continue);
    assert!(matches!(parallel.unwrap_err().cause(), Cause::Parse(_)));
}

/// GIVEN a symlink to a directory
/// WHEN it is walked
/// THEN the link target is not entered unless links are followed
#[cfg(unix)]
#[test]
fn walk_does_not_follow_symlinks_by_default() {
    let scratch = tree();
    support::unix::link(&scratch.path("sub"), &scratch.path("link"));
    support::unix::link(&scratch.path("a.txt"), &scratch.path("file-link"));

    let default = files(scratch.root(), &WalkOptions::default());
    let followed =
        files(scratch.root(), &WalkOptions { follow_links: true, ..WalkOptions::default() });

    assert_eq!(default, ["a.txt", "sub/b.txt", "sub/deep/c.txt"]);
    assert!(followed.contains(&"link/b.txt".to_owned()));
    assert!(followed.contains(&"file-link".to_owned()));
}

/// GIVEN a symlink loop and follow-links on
/// WHEN it is walked
/// THEN loops are reported without hanging
#[cfg(unix)]
#[test]
fn walk_follows_symlinks_when_asked_and_detects_loops() {
    let scratch = tree();
    support::unix::link(scratch.root(), &scratch.path("sub/back"));
    let options = WalkOptions { follow_links: true, ..WalkOptions::default() };

    let results: Vec<_> = walk_with(scratch.root(), &options).unwrap().collect();

    let errors: Vec<_> = results.iter().filter_map(|item| item.as_ref().err()).collect();
    let [error] = errors.as_slice() else { panic!("expected one error, found {errors:?}") };
    assert_eq!(error.path(), scratch.path("sub/back"));
    assert!(results.iter().filter(|item| item.is_ok()).count() >= 3);
}

/// GIVEN an unreadable directory
/// WHEN it is walked
/// THEN the error is yielded with its path
#[cfg(unix)]
#[test]
fn walk_reports_errors_instead_of_dropping_them() {
    if support::unix::is_root() {
        return;
    }
    let scratch = tree();
    let locked = scratch.dir("locked");
    support::unix::set_mode(&locked, 0o000);

    let results: Vec<_> = walk(scratch.root()).unwrap().collect();

    support::unix::set_mode(&locked, 0o700);
    let errors: Vec<_> = results.iter().filter_map(|item| item.as_ref().err()).collect();
    let [error] = errors.as_slice() else { panic!("expected one error, found {errors:?}") };
    assert_eq!(error.path(), locked);
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}

/// GIVEN a deep tree and a depth limit
/// WHEN it is walked
/// THEN nothing deeper is yielded
#[test]
fn walk_respects_max_depth() {
    let scratch = tree();

    let one = files(scratch.root(), &WalkOptions { max_depth: Some(1), ..WalkOptions::default() });
    let two = files(scratch.root(), &WalkOptions { max_depth: Some(2), ..WalkOptions::default() });

    assert_eq!(one, ["a.txt"]);
    assert_eq!(two, ["a.txt", "sub/b.txt"]);
}

/// GIVEN the sorted option
/// WHEN it is walked
/// THEN entries are in path order
#[test]
fn walk_output_is_sorted_when_requested() {
    let scratch = Scratch::new();
    for name in ["z.txt", "m/y.txt", "a.txt", "m/b.txt", "b/c.txt"] {
        scratch.write(name, "x");
    }
    let options = WalkOptions { sorted: true, ..WalkOptions::default() };

    let found: Vec<String> = walk_with(scratch.root(), &options)
        .unwrap()
        .map(|entry| relative(scratch.root(), &entry.unwrap().path))
        .collect();

    assert_eq!(found, ["a.txt", "b/c.txt", "m/b.txt", "m/y.txt", "z.txt"]);
}

/// GIVEN mixed files
/// WHEN the walk has an extension filter
/// THEN only files with a matching extension are yielded, regardless of case
#[test]
fn walk_filters_by_extension_ignoring_case() {
    let scratch = Scratch::new();
    for name in ["a.rs", "b.RS", "c.toml", "d", "sub/e.rs", "f.rs.bak", ".rs"] {
        scratch.write(name, "x");
    }
    let options = WalkOptions {
        extensions: Some(vec![".rs".into(), "toml".into()]),
        ..WalkOptions::default()
    };

    assert_eq!(files(scratch.root(), &options), ["a.rs", "b.RS", "c.toml", "sub/e.rs"]);
}

/// GIVEN a parallel walk
/// WHEN it is run
/// THEN the same set of files is visited as by a sequential walk
#[test]
fn walk_runs_in_parallel_when_asked() {
    let scratch = Scratch::new();
    for index in 0..200 {
        scratch.write(&format!("dir{}/file{index}.txt", index % 17), "x");
    }
    let options = WalkOptions { threads: Some(4), ..WalkOptions::default() };

    let parallel: Vec<String> = visited(scratch.root(), &options)
        .into_iter()
        .filter(|(_, is_dir)| !is_dir)
        .map(|(path, _)| path)
        .collect();

    assert_eq!(parallel, files(scratch.root(), &options));
    assert_eq!(parallel.len(), 200);
}

/// GIVEN a parallel walk
/// WHEN the visitor sees entries
/// THEN directories are included and the extension filter applies to files only
#[test]
fn parallel_walk_reports_directories_and_filters_files() {
    let scratch = tree();
    let options = WalkOptions { extensions: Some(vec!["txt".into()]), ..WalkOptions::default() };
    scratch.write("sub/skip.rs", "x");

    let seen = visited(scratch.root(), &options);

    let names: Vec<&str> = seen.iter().map(|(path, _)| path.as_str()).collect();
    assert_eq!(names, ["", "a.txt", "sub", "sub/b.txt", "sub/deep", "sub/deep/c.txt"]);
}

/// GIVEN a visitor that skips a directory
/// WHEN the walk is run
/// THEN nothing below it is visited
#[test]
fn parallel_walk_skips_directories_on_request() {
    let scratch = tree();
    let root = scratch.root();
    let seen = Mutex::new(Vec::new());

    walk_parallel(root, &WalkOptions::default(), || {
        let seen = &seen;
        move |item| {
            let entry = item.unwrap();
            seen.lock().unwrap().push(relative(root, &entry.path));
            if entry.path.ends_with("sub") { WalkControl::Skip } else { WalkControl::Continue }
        }
    })
    .unwrap();

    let mut seen = seen.into_inner().unwrap();
    seen.sort();
    assert_eq!(seen, ["", "a.txt", "sub"]);
}

/// GIVEN a visitor that quits on the first entry
/// WHEN a single-threaded walk is run
/// THEN no further entries are visited
#[test]
fn parallel_walk_stops_when_the_visitor_quits() {
    let scratch = tree();
    let count = Mutex::new(0usize);
    let options = WalkOptions { threads: Some(1), ..WalkOptions::default() };

    walk_parallel(scratch.root(), &options, || {
        let count = &count;
        move |_| {
            *count.lock().unwrap() += 1;
            WalkControl::Quit
        }
    })
    .unwrap();

    assert_eq!(count.into_inner().unwrap(), 1);
}

/// GIVEN extra roots
/// WHEN the trees are walked
/// THEN the files of every root are yielded, sorted within each root
#[test]
fn walk_covers_extra_roots() {
    let first = Scratch::new();
    let second = Scratch::new();
    first.write("b.txt", "x");
    first.write("a.txt", "x");
    second.write("c.txt", "x");
    let options = WalkOptions {
        extra_roots: vec![second.root().to_path_buf()],
        sorted: true,
        ..WalkOptions::default()
    };

    let sequential: Vec<_> =
        walk_with(first.root(), &options).unwrap().map(|entry| entry.unwrap().path).collect();
    let parallel = Mutex::new(Vec::new());
    walk_parallel(first.root(), &options, || {
        let parallel = &parallel;
        move |item| {
            let entry = item.unwrap();
            if entry.file_type.is_file() {
                parallel.lock().unwrap().push(entry.path);
            }
            WalkControl::Continue
        }
    })
    .unwrap();
    let mut parallel = parallel.into_inner().unwrap();
    parallel.sort();

    let mut expected = vec![first.path("a.txt"), first.path("b.txt"), second.path("c.txt")];
    assert_eq!(sequential, expected);
    expected.sort();
    assert_eq!(parallel, expected);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn special_root() -> &'static str {
    if cfg!(target_os = "linux") { "/proc" } else { "/dev" }
}

/// GIVEN a mount of proc or devfs
/// WHEN it is walked
/// THEN it is skipped unless special entries are requested
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn walk_skips_pseudo_filesystems() {
    let special = special_root();
    let below = |seen: &[(String, bool)]| {
        seen.iter()
            .filter(|(path, _)| path.starts_with(&special[1..]) && path != &special[1..])
            .count()
    };
    let skipping = WalkOptions { max_depth: Some(2), ..WalkOptions::default() };
    let including = WalkOptions { skip_special: false, ..skipping.clone() };

    let skipped = visited_readable(Path::new("/"), &skipping);
    let included = visited_readable(Path::new("/"), &including);

    assert_eq!(below(&skipped), 0);
    assert!(below(&included) > 0);
}

/// GIVEN a pseudo-filesystem passed as the root
/// WHEN it is walked
/// THEN the root itself is not skipped
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn walk_never_skips_its_root() {
    let options = WalkOptions { max_depth: Some(1), ..WalkOptions::default() };

    let seen = visited(Path::new(special_root()), &options);

    assert!(seen.len() > 1);
}
