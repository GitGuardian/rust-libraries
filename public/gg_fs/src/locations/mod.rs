//! Well-known directories.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::{self, Path, PathBuf};

use crate::error::{Cause, Context, Error, Result};
use crate::paths;

#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;
#[cfg(unix)]
mod unix;

const OP: &str = "locations";

/// A snapshot of environment variables. An empty value counts as unset.
struct Env(HashMap<OsString, OsString>);

impl Env {
    /// Returns the value of `name`, or `None` if it is unset or empty.
    fn get(&self, name: &str) -> Option<&OsString> {
        self.0.get(&imp::env_key(name.into())).filter(|value| !value.is_empty())
    }

    /// Returns the value of `name` as a path, or `None` if it is unset, empty or
    /// relative.
    fn absolute_path(&self, name: &str) -> Option<PathBuf> {
        self.get(name).map(PathBuf::from).filter(|path| path.is_absolute())
    }
}

/// A directory resolved by [`Locations::dir`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// The home directory, or `GG_USER_HOME_DIR` if set. On Unix this is the
    /// home directory of the effective user, not `$HOME`.
    Home,
    /// The configuration directory, or the override of the application if set.
    Config,
    /// The cache directory, or the override of the application if set.
    Cache,
    /// The state directory. This is the configuration directory, except for a
    /// root process whose home belongs to another user, which gets a system
    /// directory.
    State,
}

/// An application whose directories are resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct App {
    /// The name of the application directory below the platform directories.
    pub name: &'static str,
    /// The variable that overrides the configuration directory, if any.
    pub config_var: Option<&'static str>,
    /// The variable that overrides the cache directory, if any.
    pub cache_var: Option<&'static str>,
}

impl App {
    /// The `ggshield` application, overridden by `GG_CONFIG_DIR` and
    /// `GG_CACHE_DIR`.
    pub const GGSHIELD: App = App {
        name: "ggshield",
        config_var: Some("GG_CONFIG_DIR"),
        cache_var: Some("GG_CACHE_DIR"),
    };

    /// Creates an application without overrides.
    pub const fn named(name: &'static str) -> Self {
        Self { name, config_var: None, cache_var: None }
    }
}

/// Resolves well-known directories from an environment.
///
/// Overrides set to an empty string are treated as unset. No resolver returns a
/// relative path.
pub struct Locations {
    env: Env,
    app: App,
}

impl Locations {
    /// The environment variables the resolvers read for [`App::GGSHIELD`].
    pub const ENV_VARS: &'static [&'static str] = &[
        "GG_USER_HOME_DIR",
        "GG_CONFIG_DIR",
        "GG_CACHE_DIR",
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "USERPROFILE",
        "LOCALAPPDATA",
    ];

    /// Creates a resolver over the environment of the process.
    pub fn current() -> Self {
        Self::from_env(std::env::vars_os())
    }

    /// Creates a resolver over the given variables.
    ///
    /// The environment of the process is not read.
    pub fn from_env<K, V>(env: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        let env =
            env.into_iter().map(|(key, value)| (imp::env_key(key.into()), value.into())).collect();
        Self { env: Env(env), app: App::GGSHIELD }
    }

    /// Returns a resolver for `app`.
    pub fn with_app(mut self, app: App) -> Self {
        self.app = app;
        self
    }

    /// Returns the path of `dir`.
    ///
    /// # Errors
    ///
    /// This function will return [`Cause::NoHome`] if the home directory is
    /// needed and cannot be determined.
    ///
    /// # Platform-specific behavior
    ///
    /// The configuration and cache directories are named after the application in
    /// `$XDG_CONFIG_HOME` or `~/.config` and in `$XDG_CACHE_HOME` or `~/.cache`
    /// on Linux, and in `~/Library/Application Support` and `~/Library/Caches`
    /// on macOS. On Windows the configuration directory is
    /// `%LOCALAPPDATA%\GitGuardian\<application>` and the cache directory is its
    /// `Cache` subdirectory.
    pub fn dir(&self, dir: Dir) -> Result<PathBuf> {
        match dir {
            Dir::Home => self.home(),
            Dir::Config => self.config(),
            Dir::Cache => self.cache(),
            Dir::State => self.state(),
        }
    }

    fn override_dir(&self, var: Option<&str>) -> Result<Option<PathBuf>> {
        var.and_then(|var| self.env.get(var))
            .map(|value| {
                let dir = PathBuf::from(value);
                path::absolute(&dir).context(OP, &dir)
            })
            .transpose()
    }

    fn home(&self) -> Result<PathBuf> {
        match self.override_dir(Some("GG_USER_HOME_DIR"))? {
            Some(home) => Ok(home),
            None => imp::os_home(&self.env).ok_or_else(|| Error::new(OP, "~", Cause::NoHome)),
        }
    }

    fn config(&self) -> Result<PathBuf> {
        match self.override_dir(self.app.config_var)? {
            Some(config) => Ok(config),
            None => Ok(imp::config_dir(&self.env, &self.home()?, self.app.name)),
        }
    }

    fn cache(&self) -> Result<PathBuf> {
        match self.override_dir(self.app.cache_var)? {
            Some(cache) => Ok(cache),
            None => Ok(imp::cache_dir(&self.env, &self.home()?, self.app.name)),
        }
    }

    fn state(&self) -> Result<PathBuf> {
        match self.override_dir(self.app.config_var)? {
            Some(state) => Ok(state),
            None => {
                let home = self.home()?;
                let name = self.app.name;
                Ok(imp::system_state_dir(&home, name)
                    .unwrap_or_else(|| imp::config_dir(&self.env, &home, name)))
            }
        }
    }

    /// Returns the home directory of every user account.
    ///
    /// Only directories that exist and can be read are returned, without
    /// duplicates.
    ///
    /// # Platform-specific behavior
    ///
    /// On Windows the `Users` directory of every drive is searched.
    pub fn homes(&self) -> Vec<PathBuf> {
        let mut seen = HashSet::new();
        let mut homes = imp::homes();
        homes.extend(self.home().ok());
        homes.retain(|home| fs::read_dir(home).is_ok() && seen.insert(home.clone()));
        homes
    }

    /// Expands a leading `~` to the home directory.
    ///
    /// A path that does not start with `~`, including the `~user` form, is
    /// returned unchanged.
    ///
    /// # Errors
    ///
    /// This function will return an error if the path starts with `~` and the
    /// home directory cannot be determined.
    pub fn expand_tilde<'a>(&self, path: &'a Path) -> Result<Cow<'a, Path>> {
        match path.to_str().and_then(split_tilde) {
            Some(rest) => {
                let home = self.home()?;
                let expanded = if rest.is_empty() { home.clone() } else { home.join(rest) };
                if paths::is_within(&home, &expanded) {
                    Ok(Cow::Owned(expanded))
                } else {
                    Ok(Cow::Borrowed(path))
                }
            }
            None => Ok(Cow::Borrowed(path)),
        }
    }
}

/// Returns the nearest ancestor of `start`, including `start`, that contains a
/// `.git` file or directory.
pub fn project_root(start: impl AsRef<Path>) -> Option<PathBuf> {
    let start = path::absolute(start.as_ref()).ok()?;
    start.ancestors().find(|dir| dir.join(".git").exists()).map(Path::to_path_buf)
}

fn split_tilde(path: &str) -> Option<&str> {
    let rest = path.strip_prefix('~')?;
    if rest.is_empty() {
        Some(rest)
    } else {
        rest.strip_prefix(['/', '\\']).map(|rest| rest.trim_start_matches(['/', '\\']))
    }
}
