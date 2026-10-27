mod support;

use std::path::{Path, PathBuf};

use gg_fs::locations::{App, Dir, Locations, project_root};
use support::Scratch;

fn locations<'a>(env: impl IntoIterator<Item = (&'a str, &'a Path)>) -> Locations {
    Locations::from_env(
        env.into_iter().map(|(key, value)| (key.to_owned(), value.as_os_str().to_owned())),
    )
}

fn with_home(home: &Path) -> Locations {
    locations([("GG_USER_HOME_DIR", home)])
}

/// GIVEN GG_CONFIG_DIR is set to a directory
/// WHEN the config dir is resolved
/// THEN that directory is returned regardless of platform
#[test]
fn config_dir_honours_gg_config_dir_override() {
    let scratch = Scratch::new();
    let config = scratch.path("custom-config");

    let resolved = locations([("GG_CONFIG_DIR", config.as_path())]).dir(Dir::Config).unwrap();

    assert_eq!(resolved, config);
}

/// GIVEN GG_CACHE_DIR is set to a directory
/// WHEN the cache dir is resolved
/// THEN that directory is returned
#[test]
fn cache_dir_honours_gg_cache_dir_override() {
    let scratch = Scratch::new();
    let cache = scratch.path("custom-cache");

    let resolved = locations([("GG_CACHE_DIR", cache.as_path())]).dir(Dir::Cache).unwrap();

    assert_eq!(resolved, cache);
}

/// GIVEN no override on Linux
/// WHEN the dirs are resolved
/// THEN they are ggshield under the XDG directory or the dot directories of the home
#[cfg(target_os = "linux")]
#[test]
fn dirs_default_to_ggshield_under_xdg_or_home() {
    let scratch = Scratch::new();
    let home = scratch.dir("home");
    let xdg_config = scratch.path("xdg-config");
    let xdg_cache = scratch.path("xdg-cache");

    let plain = with_home(&home);
    let xdg = locations([
        ("GG_USER_HOME_DIR", home.as_path()),
        ("XDG_CONFIG_HOME", xdg_config.as_path()),
        ("XDG_CACHE_HOME", xdg_cache.as_path()),
    ]);

    assert_eq!(plain.dir(Dir::Config).unwrap(), home.join(".config/ggshield"));
    assert_eq!(plain.dir(Dir::Cache).unwrap(), home.join(".cache/ggshield"));
    assert_eq!(xdg.dir(Dir::Config).unwrap(), xdg_config.join("ggshield"));
    assert_eq!(xdg.dir(Dir::Cache).unwrap(), xdg_cache.join("ggshield"));
}

/// GIVEN no override on macOS
/// WHEN the dirs are resolved
/// THEN they are ggshield under the Library directories of the home
#[cfg(target_os = "macos")]
#[test]
fn dirs_default_to_ggshield_under_library() {
    let scratch = Scratch::new();
    let home = scratch.dir("home");

    let locations = with_home(&home);

    assert_eq!(
        locations.dir(Dir::Config).unwrap(),
        home.join("Library/Application Support/ggshield")
    );
    assert_eq!(locations.dir(Dir::Cache).unwrap(), home.join("Library/Caches/ggshield"));
}

/// GIVEN no override on Windows
/// WHEN the dirs are resolved
/// THEN the config dir is GitGuardian/ggshield under the local app data directory and the
/// cache dir is the Cache folder beside it
#[cfg(windows)]
#[test]
fn dirs_on_windows_use_local_appdata() {
    let scratch = Scratch::new();
    let home = scratch.dir("home");
    let local = scratch.dir("local");
    let locations =
        locations([("GG_USER_HOME_DIR", home.as_path()), ("LOCALAPPDATA", local.as_path())]);

    let config = locations.dir(Dir::Config).unwrap();

    assert_eq!(config, local.join("GitGuardian").join("ggshield"));
    assert_eq!(locations.dir(Dir::Cache).unwrap(), config.join("Cache"));
}

/// GIVEN overrides set to empty strings
/// WHEN the dirs are resolved
/// THEN the platform defaults are used
#[test]
fn empty_override_is_treated_as_unset() {
    let scratch = Scratch::new();
    let home = scratch.dir("home");
    let empty = Path::new("");

    let with_empty = locations([
        ("GG_USER_HOME_DIR", home.as_path()),
        ("GG_CONFIG_DIR", empty),
        ("GG_CACHE_DIR", empty),
    ]);
    let without = with_home(&home);

    for dir in [Dir::Config, Dir::Cache, Dir::State] {
        let resolved = with_empty.dir(dir).unwrap();
        assert_eq!(resolved, without.dir(dir).unwrap(), "{dir:?}");
        assert!(resolved.is_absolute());
    }
}

/// GIVEN a relative override
/// WHEN the dir is resolved
/// THEN the result is made absolute
#[test]
fn relative_override_is_made_absolute() {
    let resolved =
        locations([("GG_CONFIG_DIR", Path::new("relative/config"))]).dir(Dir::Config).unwrap();

    assert!(resolved.is_absolute());
    assert!(resolved.ends_with("relative/config"));
}

/// GIVEN GG_USER_HOME_DIR is set
/// WHEN the home dir is resolved
/// THEN the override wins over the platform home
#[test]
fn home_dir_honours_gg_user_home_dir_override() {
    let scratch = Scratch::new();
    let home = scratch.dir("custom-home");

    assert_eq!(with_home(&home).dir(Dir::Home).unwrap(), home);
}

/// GIVEN HOME is stale, as under su -m or sudo -E
/// WHEN the home dir is resolved
/// THEN the passwd entry of the effective uid is used
#[cfg(unix)]
#[test]
fn home_dir_prefers_passwd_entry_of_effective_uid() {
    let stale = Path::new("/stale/home");
    let other = Path::new("/another/stale/home");

    let first = locations([("HOME", stale)]).dir(Dir::Home).unwrap();
    let second = locations([("HOME", other)]).dir(Dir::Home).unwrap();

    assert_eq!(first, second);
    assert_ne!(first, stale);
    assert!(first.is_absolute());
}

/// GIVEN a custom application
/// WHEN its dirs are resolved
/// THEN they are named after it and the ggshield overrides are not read
#[test]
fn custom_app_uses_its_own_name_and_ignores_ggshield_overrides() {
    let scratch = Scratch::new();
    let home = scratch.dir("home");
    let ignored = scratch.path("ignored");
    let locations = locations([
        ("GG_USER_HOME_DIR", home.as_path()),
        ("GG_CONFIG_DIR", ignored.as_path()),
        ("GG_CACHE_DIR", ignored.as_path()),
    ])
    .with_app(App::named("tool"));

    let config = locations.dir(Dir::Config).unwrap();
    let cache = locations.dir(Dir::Cache).unwrap();

    let cache_suffix =
        if cfg!(windows) { Path::new("tool").join("Cache") } else { PathBuf::from("tool") };
    assert_eq!(config.file_name().unwrap(), "tool");
    assert!(cache.ends_with(&cache_suffix), "{}", cache.display());
    assert_ne!(config, ignored);
    assert_ne!(cache, ignored);
}

/// GIVEN a config override and no override
/// WHEN the state dir is resolved
/// THEN the override wins, and otherwise it is the config dir for a normal user
#[cfg(unix)]
#[test]
fn state_dir_is_the_config_dir_for_a_normal_user() {
    if support::unix::is_root() {
        return;
    }
    let scratch = Scratch::new();
    let home = scratch.dir("home");
    let config = scratch.path("config");

    let overridden = locations([("GG_CONFIG_DIR", config.as_path())]);
    let plain = with_home(&home);

    assert_eq!(overridden.dir(Dir::State).unwrap(), config);
    assert_eq!(plain.dir(Dir::State).unwrap(), plain.dir(Dir::Config).unwrap());
}

/// GIVEN the set of path override variables
/// WHEN the list is requested
/// THEN it contains every variable the resolvers read
#[test]
fn env_override_names_are_listed_for_scrubbing() {
    for name in ["GG_USER_HOME_DIR", "GG_CONFIG_DIR", "GG_CACHE_DIR", "HOME", "XDG_CONFIG_HOME"] {
        assert!(Locations::ENV_VARS.contains(&name), "{name}");
    }
}

/// GIVEN an existing and a missing home override
/// WHEN all homes are listed
/// THEN only the existing home is returned, once
#[test]
fn all_users_homes_are_enumerated_once() {
    let scratch = Scratch::new();
    let home = scratch.dir("home");
    let missing = scratch.path("missing");

    let homes = with_home(&home).homes();

    assert_eq!(homes.iter().filter(|candidate| **candidate == home).count(), 1);
    assert!(!with_home(&missing).homes().contains(&missing));
    let mut unique = homes.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), homes.len());
}

/// GIVEN a path starting with a tilde
/// WHEN it is expanded
/// THEN the resolved home replaces the tilde and ~user forms are left alone
#[test]
fn tilde_expands_to_resolved_home() {
    let scratch = Scratch::new();
    let home = scratch.dir("home");
    let locations = with_home(&home);
    let expand = |input: &str| locations.expand_tilde(Path::new(input)).unwrap().into_owned();

    assert_eq!(expand("~"), home);
    assert_eq!(expand("~/docs/a.txt"), home.join("docs").join("a.txt"));
    assert_eq!(expand("~\\docs"), home.join("docs"));
    assert_eq!(expand("~//docs"), home.join("docs"));
    assert_eq!(expand("~alice/docs"), PathBuf::from("~alice/docs"));
    assert_eq!(expand("docs/~"), PathBuf::from("docs/~"));
    assert_eq!(expand("/abs/path"), PathBuf::from("/abs/path"));
}

/// GIVEN a tilde path that climbs out of the home
/// WHEN it is expanded
/// THEN it is returned unchanged
#[test]
fn tilde_expansion_cannot_escape_the_home() {
    let scratch = Scratch::new();
    let home = scratch.dir("home");

    let expanded = with_home(&home).expand_tilde(Path::new("~/../outside")).unwrap();

    assert_eq!(expanded, Path::new("~/../outside"));
}

/// GIVEN a nested directory inside a checkout
/// WHEN the project root is discovered
/// THEN the nearest ancestor containing .git as file or directory is returned
#[test]
fn project_root_is_found_by_walking_up_to_git() {
    let scratch = Scratch::new();
    scratch.dir("repo/.git");
    let nested = scratch.dir("repo/src/deep");
    scratch.write("worktree/.git", "gitdir: ../repo/.git");
    let worktree_nested = scratch.dir("worktree/pkg");
    scratch.dir("outer/.git");
    scratch.dir("outer/inner/.git");

    assert_eq!(project_root(&nested), Some(scratch.path("repo")));
    assert_eq!(project_root(scratch.path("repo")), Some(scratch.path("repo")));
    assert_eq!(project_root(&worktree_nested), Some(scratch.path("worktree")));
    assert_eq!(project_root(scratch.path("outer/inner")), Some(scratch.path("outer/inner")));
}

/// GIVEN a directory outside any checkout
/// WHEN the project root is discovered
/// THEN none is returned
#[test]
fn project_root_is_none_outside_a_checkout() {
    let scratch = Scratch::new();
    let plain = scratch.dir("plain/dir");

    assert_eq!(project_root(&plain), None);
}
