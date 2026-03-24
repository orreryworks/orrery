//! Filesystem-backed [`SourceProvider`] for the CLI environment.
//!
//! [`FsSourceProvider`] resolves import paths relative to the importing file
//! and reads source text from the filesystem. Paths are canonicalized
//! via [`fs::canonicalize`] so that the resolver can deduplicate files by
//! comparing canonical [`PathBuf`] keys.

use std::{
    fs,
    path::{Path, PathBuf},
};

use orrery_parser::{error::SourceError, source_provider::SourceProvider};

/// Filesystem-backed [`SourceProvider`].
///
/// - [`resolve_path`](SourceProvider::resolve_path): joins the parent directory
///   of `from` with `import_path`, appends `.orr`, and canonicalizes the result.
/// - [`read_source`](SourceProvider::read_source): delegates to
///   [`fs::read_to_string`].
#[derive(Debug, Clone, Copy, Default)]
pub struct FsSourceProvider;

impl FsSourceProvider {
    /// Creates a new filesystem source provider.
    pub fn new() -> Self {
        Self
    }
}

impl SourceProvider for FsSourceProvider {
    fn resolve_path(&self, from: &Path, import_path: &str) -> Result<PathBuf, SourceError> {
        let dir = from.parent().unwrap_or_else(|| Path::new("."));

        let mut target = dir.join(import_path);
        target.set_extension("orr");

        fs::canonicalize(&target).map_err(|err| SourceError::new(&target, format!("{err}")))
    }

    fn read_source(&self, path: &Path) -> Result<String, SourceError> {
        fs::read_to_string(path).map_err(|err| SourceError::new(path, format!("{err}")))
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Helper: write a file inside `dir`, creating parent directories as needed.
    fn write_file(dir: &Path, relative: &str, content: &str) -> PathBuf {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn resolve_returns_canonical_path() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "a.orr", "library;");
        let main = write_file(tmp.path(), "main.orr", "");

        let provider = FsSourceProvider::new();
        let resolved = provider.resolve_path(&main, "a").unwrap();

        // Canonical path should be absolute with no `.` or `..` segments.
        assert!(resolved.is_absolute());
    }

    #[test]
    fn resolve_appends_orr_extension() {
        let tmp = TempDir::new().unwrap();
        let orr = write_file(tmp.path(), "lib.orr", "library;");
        let main = write_file(tmp.path(), "main.orr", "");

        let provider = FsSourceProvider::new();
        let resolved = provider.resolve_path(&main, "lib").unwrap();

        // Resolved path should point to the same file as the .orr we wrote.
        assert_eq!(fs::canonicalize(&orr).unwrap(), resolved);
    }

    #[test]
    fn resolve_and_read_same_directory() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "styles.orr", "library;");
        let main = write_file(tmp.path(), "main.orr", "diagram component;");

        let provider = FsSourceProvider::new();
        let resolved = provider.resolve_path(&main, "styles").unwrap();
        let source = provider.read_source(&resolved).unwrap();
        assert_eq!(source, "library;");
    }

    #[test]
    fn resolve_subdirectory() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "shared/styles.orr", "library;");
        let main = write_file(tmp.path(), "main.orr", "");

        let provider = FsSourceProvider::new();
        let resolved = provider.resolve_path(&main, "shared/styles").unwrap();
        let source = provider.read_source(&resolved).unwrap();
        assert_eq!(source, "library;");
    }

    #[test]
    fn resolve_parent_directory() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "shared/base.orr", "library;");
        let nested = write_file(tmp.path(), "diagrams/main.orr", "");

        let provider = FsSourceProvider::new();
        let resolved = provider.resolve_path(&nested, "../shared/base").unwrap();
        let source = provider.read_source(&resolved).unwrap();
        assert_eq!(source, "library;");
    }

    #[test]
    fn resolve_missing_file_is_error() {
        let tmp = TempDir::new().unwrap();
        let main = write_file(tmp.path(), "main.orr", "");

        let provider = FsSourceProvider::new();
        let err = provider.resolve_path(&main, "nonexistent").unwrap_err();
        assert!(err.message().contains("No such file"));
    }

    #[test]
    fn read_missing_file_is_error() {
        let tmp = TempDir::new().unwrap();
        let provider = FsSourceProvider::new();
        let err = provider
            .read_source(&tmp.path().join("orrery_definitely_missing_file.orr"))
            .unwrap_err();
        assert!(err.message().contains("No such file"));
    }

    #[test]
    fn deduplication_via_canonical_paths() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "shared/styles.orr", "library;");
        let main = write_file(tmp.path(), "main.orr", "");
        let nested = write_file(tmp.path(), "sub/deep.orr", "");

        let provider = FsSourceProvider::new();

        // Two different relative paths that point to the same file.
        let from_main = provider.resolve_path(&main, "shared/styles").unwrap();
        let from_nested = provider.resolve_path(&nested, "../shared/styles").unwrap();

        assert_eq!(from_main, from_nested);
    }

    #[test]
    fn chained_resolution() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "base.orr", "library;");
        write_file(tmp.path(), "ext.orr", "library;");
        let main = write_file(tmp.path(), "main.orr", "");

        let provider = FsSourceProvider::new();

        // main.orr → ext.orr → base.orr
        let ext = provider.resolve_path(&main, "ext").unwrap();
        let base = provider.resolve_path(&ext, "base").unwrap();
        let source = provider.read_source(&base).unwrap();
        assert_eq!(source, "library;");
    }

    #[test]
    fn resolve_absolute_import_path() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "libs/base.orr", "library;");
        let main = write_file(tmp.path(), "deeply/nested/main.orr", "");

        let provider = FsSourceProvider::new();

        // Absolute import path — the `from` file's directory should be ignored.
        // E.g. main.orr: `import "/absolute/path/to/base";`
        let abs_import = tmp.path().join("libs/base");
        let resolved = provider
            .resolve_path(&main, abs_import.to_str().unwrap())
            .unwrap();

        assert!(resolved.is_absolute());
        let source = provider.read_source(&resolved).unwrap();
        assert_eq!(source, "library;");
    }
}
