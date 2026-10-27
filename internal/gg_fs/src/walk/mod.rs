use std::ffi::OsStr;
use std::fs::FileType;
use std::io;
use std::path::{Path, PathBuf};

use ignore::overrides::{Override, OverrideBuilder};
use ignore::{WalkBuilder, WalkState};

use crate::error::{Cause, Error, Result};

const OP: &str = "walk";

#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;

/// An entry found by [`walk`], [`walk_with`] or [`walk_parallel`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DirEntry {
    /// The path of the entry, including the walk root.
    pub path: PathBuf,
    /// The type of the entry.
    pub file_type: FileType,
}

/// What a [`walk_parallel`] visitor asks the walk to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkControl {
    /// Continues the walk.
    Continue,
    /// Does not descend into the directory. Has no effect on other entries.
    Skip,
    /// Stops the walk as soon as possible. Other threads may still deliver
    /// entries.
    Quit,
}

/// Options for [`walk_with`] and [`walk_parallel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkOptions {
    /// Whether to follow symbolic links. A link that is not followed is not
    /// yielded by [`walk_with`] and is reported as a symbolic link entry by
    /// [`walk_parallel`]. Loops are yielded as errors. Defaults to `false`.
    pub follow_links: bool,
    /// Whether to skip paths matched by `.gitignore` files. Defaults to `true`.
    pub respect_gitignore: bool,
    /// Roots walked after the root given to the walk. Defaults to empty.
    pub extra_roots: Vec<PathBuf>,
    /// Glob overrides, relative to the root given to the walk. A glob includes
    /// the paths it matches, and a glob with a leading `!` excludes them. When
    /// any including glob is present, paths that match no glob are skipped.
    /// Overrides take precedence over ignore files. Defaults to empty.
    pub overrides: Vec<String>,
    /// The maximum depth below each root. `Some(1)` lists a single directory.
    /// Defaults to `None`.
    pub max_depth: Option<usize>,
    /// If set, a file is yielded only if it has one of these extensions.
    /// Extensions are written without the dot and compared case-insensitively
    /// against the last extension of the file name. Entries that are not files are
    /// not affected. Defaults to `None`.
    pub extensions: Option<Vec<String>>,
    /// Whether [`walk_with`] sorts entries by path within each root, walking the
    /// roots in the order given. [`walk_parallel`] ignores it. Defaults to
    /// `false`.
    pub sorted: bool,
    /// The number of threads used by [`walk_parallel`]. `None` lets the walker
    /// choose. Defaults to `None`.
    pub threads: Option<usize>,
    /// Whether to skip pseudo-filesystems such as `proc` and `sysfs`, and on
    /// Windows cloud placeholder files and reparse points that are not symbolic
    /// links or junctions. The roots are never skipped. Defaults to `true`.
    pub skip_special: bool,
}

impl Default for WalkOptions {
    /// Returns options that do not follow symbolic links, respect `.gitignore`
    /// files and skip special entries.
    fn default() -> Self {
        Self {
            follow_links: false,
            respect_gitignore: true,
            extra_roots: Vec::new(),
            overrides: Vec::new(),
            max_depth: None,
            extensions: None,
            sorted: false,
            threads: None,
            skip_special: true,
        }
    }
}

/// An iterator over the files in directory trees.
///
/// This struct is created by [`walk`] and [`walk_with`]. Each item is a
/// [`DirEntry`] for a regular file, or an [`Error`] for a
/// directory that could not be read.
pub struct Walk {
    walk: ignore::Walk,
    filter: Filter,
}

impl Iterator for Walk {
    type Item = Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        self.walk.find_map(|item| match self.filter.apply(item)? {
            Ok(entry) if !entry.file_type.is_file() => None,
            other => Some(other),
        })
    }
}

/// Returns an iterator over the files below `root`.
///
/// This is [`walk_with`] with default [`WalkOptions`].
pub fn walk(root: impl AsRef<Path>) -> Result<Walk> {
    walk_with(root, &WalkOptions::default())
}

/// Returns an iterator over the files below `root` and
/// [`WalkOptions::extra_roots`], with options.
///
/// Hidden files and the contents of `.git` directories are included.
///
/// # Errors
///
/// This function will return [`Cause::Parse`] if an override glob is invalid.
pub fn walk_with(root: impl AsRef<Path>, options: &WalkOptions) -> Result<Walk> {
    Walker::try_new(root, options).map(Walker::walk)
}

/// Walks the trees below `root` and [`WalkOptions::extra_roots`] on several
/// threads.
///
/// `make_visitor` is called once for each thread. Each visitor receives every
/// entry found by its thread, including directories and errors, in no fixed
/// order. This function returns when the walk is finished.
///
/// # Errors
///
/// This function will return [`Cause::Parse`] if an override glob is invalid.
pub fn walk_parallel<V>(
    root: impl AsRef<Path>,
    options: &WalkOptions,
    make_visitor: impl FnMut() -> V,
) -> Result<()>
where
    V: FnMut(Result<DirEntry>) -> WalkControl + Send,
{
    Walker::try_new(root, options).map(|walker| walker.run(make_visitor))
}

impl From<WalkControl> for WalkState {
    fn from(control: WalkControl) -> Self {
        match control {
            WalkControl::Continue => Self::Continue,
            WalkControl::Skip => Self::Skip,
            WalkControl::Quit => Self::Quit,
        }
    }
}

struct Walker {
    builder: WalkBuilder,
    filter: Filter,
}

impl Walker {
    fn try_new(root: impl AsRef<Path>, options: &WalkOptions) -> Result<Self> {
        let root = root.as_ref();
        let mut builder = WalkBuilder::new(root);
        for extra in &options.extra_roots {
            builder.add(extra);
        }
        builder
            .follow_links(options.follow_links)
            .max_depth(options.max_depth)
            .standard_filters(false)
            .parents(true)
            .git_ignore(options.respect_gitignore)
            .require_git(false)
            .overrides(overrides(root, &options.overrides)?);
        if let Some(threads) = options.threads {
            builder.threads(threads);
        }
        if options.sorted {
            builder.sort_by_file_path(Path::cmp);
        }
        if options.skip_special {
            builder.filter_entry(is_ordinary);
        }
        Ok(Self { builder, filter: Filter::new(root, options) })
    }

    fn walk(self) -> Walk {
        Walk { walk: self.builder.build(), filter: self.filter }
    }

    fn run<V>(self, mut make_visitor: impl FnMut() -> V)
    where
        V: FnMut(Result<DirEntry>) -> WalkControl + Send,
    {
        let filter = &self.filter;
        self.builder.build_parallel().run(|| {
            let mut visitor = make_visitor();
            Box::new(move |item| {
                filter.apply(item).map_or(WalkState::Continue, |item| visitor(item).into())
            })
        });
    }
}

fn overrides(root: &Path, globs: &[String]) -> Result<Override> {
    let mut builder = OverrideBuilder::new(root);
    for glob in globs {
        builder.add(glob).map_err(|err| glob_error(root, err))?;
    }
    builder.build().map_err(|err| glob_error(root, err))
}

fn is_ordinary(entry: &ignore::DirEntry) -> bool {
    !entry.file_type().is_some_and(|file_type| imp::is_special(entry.path(), file_type))
}

struct Filter {
    root: PathBuf,
    extensions: Option<Vec<String>>,
}

impl Filter {
    fn new(root: &Path, options: &WalkOptions) -> Self {
        let extensions = options.extensions.as_ref().map(|extensions| {
            extensions
                .iter()
                .map(|extension| extension.trim_start_matches('.').to_owned())
                .collect()
        });
        Self { root: root.to_path_buf(), extensions }
    }

    fn apply(
        &self,
        item: std::result::Result<ignore::DirEntry, ignore::Error>,
    ) -> Option<Result<DirEntry>> {
        match item {
            Ok(entry) => {
                let file_type = entry.file_type()?;
                (!file_type.is_file() || self.has_extension(entry.path()))
                    .then(|| Ok(DirEntry { path: entry.into_path(), file_type }))
            }
            Err(err) => Some(Err(convert(&self.root, err))),
        }
    }

    fn has_extension(&self, path: &Path) -> bool {
        self.extensions.as_ref().is_none_or(|extensions| {
            path.extension().and_then(OsStr::to_str).is_some_and(|extension| {
                extensions.iter().any(|wanted| wanted.eq_ignore_ascii_case(extension))
            })
        })
    }
}

fn glob_error(root: &Path, err: ignore::Error) -> Error {
    Error::new(OP, root, Cause::Parse(Box::new(err)))
}

fn convert(root: &Path, err: ignore::Error) -> Error {
    let path = error_path(&err).unwrap_or(root).to_path_buf();
    let message = err.to_string();
    let cause = err.into_io_error().unwrap_or_else(|| io::Error::other(message));
    Error::new(OP, path, Cause::Io(cause))
}

fn error_path(err: &ignore::Error) -> Option<&Path> {
    match err {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::Loop { child, .. } => Some(child),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            error_path(err)
        }
        ignore::Error::Partial(errors) => errors.iter().find_map(error_path),
        _ => None,
    }
}
