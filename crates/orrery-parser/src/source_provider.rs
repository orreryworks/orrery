//! Source provider abstraction for file I/O.
//!
//! This module decouples the import resolver from the filesystem, enabling
//! different environments (CLI, tests, LSP, WASM) to supply their own
//! file-reading strategy.
//!
//! # Overview
//!
//! - [`SourceProvider`] — trait with three methods: [`resolve_path`](SourceProvider::resolve_path),
//!   [`read_source`](SourceProvider::read_source), and [`derive_namespace`](SourceProvider::derive_namespace).
//! - [`SourceError`] — lightweight, `Clone`-able error returned by providers
//!   (defined in the [`error`](crate::error) module).

use std::path::{Path, PathBuf};

use orrery_core::identifier::Id;

use crate::error::SourceError;

/// Abstraction over file I/O for the import resolver.
///
/// Implementations provide environment-specific file resolution and reading.
/// The trait is object-safe so the resolver can use `&dyn SourceProvider`.
pub trait SourceProvider {
    /// Resolve an import path relative to the importing file.
    ///
    /// The implementation must:
    /// 1. Determine the directory of `from` (the importing file).
    /// 2. Join it with `import_path`.
    /// 3. Append the `.orr` extension.
    /// 4. Return a normalized/canonical path for deduplication.
    ///
    /// # Arguments
    ///
    /// * `from` — Path of the file containing the `import` statement.
    /// * `import_path` — The raw path string from source (e.g., `"shared/styles"`).
    ///
    /// # Errors
    ///
    /// Returns [`SourceError`] if the path cannot be resolved.
    fn resolve_path(&self, from: &Path, import_path: &str) -> Result<PathBuf, SourceError>;

    /// Read the source text of a file at the given path.
    ///
    /// The `path` argument should be a value previously returned by
    /// [`resolve_path`](Self::resolve_path).
    ///
    /// # Errors
    ///
    /// Returns [`SourceError`] if the file cannot be read.
    fn read_source(&self, path: &Path) -> Result<String, SourceError>;

    /// Derives a namespace [`Id`] from an import path.
    ///
    /// The default implementation extracts the final component's file stem
    /// (e.g. `shared/styles` → `styles`, `../common/base.orr` → `base`).
    ///
    /// # Arguments
    ///
    /// * `import_path` — The import path as a [`Path`] reference.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError`] if a valid namespace name cannot be derived
    /// from the import path (e.g. the path has no file stem or contains
    /// non-UTF-8 characters).
    fn derive_namespace(&self, import_path: &Path) -> Result<Id, SourceError> {
        let name = import_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                SourceError::new(
                    import_path,
                    "cannot derive namespace: path has no valid file stem",
                )
            })?;
        Ok(Id::new(name))
    }
}

/// An in-memory [`SourceProvider`] is a test-only provider backed by a `HashMap`.
///
/// This is a minimal test helper. Keys are stored and looked up **exactly**
/// as provided — there is no path normalization. The test writer controls
/// both the registered keys and the import paths, so they are responsible
/// for making them match.
///
/// `resolve_path` joins the parent directory of `from` with `import_path`,
/// appends `.orr`, and does a direct HashMap lookup on the result.
#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub(crate) struct InMemorySourceProvider {
    files: std::collections::HashMap<PathBuf, String>,
}

#[cfg(test)]
impl InMemorySourceProvider {
    /// Creates a new, empty in-memory source provider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a file in this provider.
    pub fn add_file(&mut self, path: impl Into<PathBuf>, source: impl Into<String>) {
        self.files.insert(path.into(), source.into());
    }
}

#[cfg(test)]
impl SourceProvider for InMemorySourceProvider {
    fn resolve_path(&self, from: &Path, import_path: &str) -> Result<PathBuf, SourceError> {
        let dir = from.parent().unwrap_or_else(|| Path::new(""));

        let mut target = dir.join(import_path);
        target.set_extension("orr");

        if self.files.contains_key(&target) {
            Ok(target)
        } else {
            Err(SourceError::new(
                &target,
                format!("file not found: {}", target.display()),
            ))
        }
    }

    fn read_source(&self, path: &Path) -> Result<String, SourceError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| SourceError::new(path, format!("file not found: {}", path.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_path_same_directory() {
        let mut provider = InMemorySourceProvider::new();
        provider.add_file("styles.orr", "library;");

        let resolved = provider
            .resolve_path(Path::new("main.orr"), "styles")
            .unwrap();
        assert_eq!(resolved, PathBuf::from("styles.orr"));
    }

    #[test]
    fn resolve_path_subdirectory() {
        let mut provider = InMemorySourceProvider::new();
        provider.add_file("shared/styles.orr", "library;");

        let resolved = provider
            .resolve_path(Path::new("main.orr"), "shared/styles")
            .unwrap();
        assert_eq!(resolved, PathBuf::from("shared/styles.orr"));
    }

    #[test]
    fn resolve_path_from_nested_file() {
        let mut provider = InMemorySourceProvider::new();
        provider.add_file("shared/base.orr", "library;");

        // shared/ext.orr imports "base" → resolves to shared/base.orr
        let resolved = provider
            .resolve_path(Path::new("shared/ext.orr"), "base")
            .unwrap();
        assert_eq!(resolved, PathBuf::from("shared/base.orr"));
    }

    #[test]
    fn resolve_path_file_not_found() {
        let provider = InMemorySourceProvider::new();

        let err = provider
            .resolve_path(Path::new("main.orr"), "missing")
            .unwrap_err();
        assert_eq!(err.path(), Path::new("missing.orr"));
        assert!(err.message().contains("file not found"));
    }

    #[test]
    fn resolve_path_appends_orr_extension() {
        let mut provider = InMemorySourceProvider::new();
        provider.add_file("lib.orr", "library;");

        let resolved = provider.resolve_path(Path::new("main.orr"), "lib").unwrap();
        assert_eq!(resolved, PathBuf::from("lib.orr"));
    }

    #[test]
    fn read_source_existing_file() {
        let mut provider = InMemorySourceProvider::new();
        provider.add_file("styles.orr", "library;\ntype Box = Rectangle;");

        let source = provider.read_source(Path::new("styles.orr")).unwrap();
        assert_eq!(source, "library;\ntype Box = Rectangle;");
    }

    #[test]
    fn read_source_missing_file() {
        let provider = InMemorySourceProvider::new();

        let err = provider.read_source(Path::new("missing.orr")).unwrap_err();
        assert_eq!(err.path(), Path::new("missing.orr"));
        assert!(err.message().contains("file not found"));
    }

    #[test]
    fn resolve_then_read_round_trip() {
        let mut provider = InMemorySourceProvider::new();
        provider.add_file("shared/styles.orr", "library;\ntype S = Rectangle;");
        provider.add_file("main.orr", "diagram component;");

        let resolved = provider
            .resolve_path(Path::new("main.orr"), "shared/styles")
            .unwrap();
        let source = provider.read_source(&resolved).unwrap();
        assert!(source.contains("type S = Rectangle"));
    }

    #[test]
    fn resolve_chained_imports() {
        let mut provider = InMemorySourceProvider::new();
        provider.add_file("base.orr", "library;");
        provider.add_file("ext.orr", "library;");
        provider.add_file("main.orr", "diagram component;");

        // main.orr → ext → base
        let ext_path = provider.resolve_path(Path::new("main.orr"), "ext").unwrap();
        assert_eq!(ext_path, PathBuf::from("ext.orr"));

        let base_path = provider.resolve_path(&ext_path, "base").unwrap();
        assert_eq!(base_path, PathBuf::from("base.orr"));

        let base_source = provider.read_source(&base_path).unwrap();
        assert_eq!(base_source, "library;");
    }

    #[test]
    fn resolve_nested_directory_structure() {
        let mut provider = InMemorySourceProvider::new();
        provider.add_file("shared/base/types.orr", "library;");
        provider.add_file("shared/ext.orr", "library;");

        // shared/ext.orr imports base/types → shared/base/types.orr
        let types = provider
            .resolve_path(Path::new("shared/ext.orr"), "base/types")
            .unwrap();
        assert_eq!(types, PathBuf::from("shared/base/types.orr"));
    }

    #[test]
    fn empty_import_path_is_error() {
        let provider = InMemorySourceProvider::new();

        let err = provider
            .resolve_path(Path::new("main.orr"), "")
            .unwrap_err();
        assert!(err.message().contains("file not found"));
    }

    #[test]
    fn derive_namespace() {
        let provider = InMemorySourceProvider::new();
        let id = provider.derive_namespace(Path::new("simple")).unwrap();
        assert!(id == "simple");
        
        let id = provider
            .derive_namespace(Path::new("shared/nested"))
            .unwrap();
        assert!(id == "nested");
        
        let id = provider
            .derive_namespace(Path::new("../relative/path"))
            .unwrap();
        assert!(id == "path");
        
        let id = provider
            .derive_namespace(Path::new("shared/extension.orr"))
            .unwrap();
        assert!(id == "extension");
    }

    #[test]
    fn derive_namespace_empty_path_is_error() {
        let provider = InMemorySourceProvider::new();
        let err = provider.derive_namespace(Path::new("")).unwrap_err();
        assert!(
            err.message().contains("cannot derive namespace"),
            "expected namespace error, got: {}",
            err.message()
        );
    }

    #[test]
    fn overwrite_file() {
        let mut provider = InMemorySourceProvider::new();
        provider.add_file("a.orr", "version 1");
        provider.add_file("a.orr", "version 2");

        let source = provider.read_source(Path::new("a.orr")).unwrap();
        assert_eq!(source, "version 2");
    }
}
