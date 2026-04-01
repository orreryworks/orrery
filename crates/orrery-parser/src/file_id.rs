//! File identifier backed by [`interner`].
//!
//! This module provides [`FileId`], a cheap handle representing a canonical
//! file path. It wraps a [`Symbol`] from the shared [`interner`] module.

use std::path::Path;

use orrery_core::interner::{self, Symbol};

/// Cheap handle representing a canonical file path.
///
/// Wraps a [`Symbol`] from the shared global interner. Two `FileId`s
/// that were created from the same path string are guaranteed to be equal
/// (O(1) comparison via symbol equality).
///
/// # Examples
///
/// ```text
/// # use std::path::Path;
/// # use orrery_parser::file_id::FileId;
/// let id = FileId::new(Path::new("shared/styles.orr"));
/// assert_eq!(id.as_str(), "shared/styles.orr");
///
/// // Same path produces the same id.
/// let id2 = FileId::new(Path::new("shared/styles.orr"));
/// assert_eq!(id, id2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(Symbol);

impl FileId {
    /// Creates a [`FileId`] by interning the string representation of `path`.
    ///
    /// If the path was already interned, the existing symbol is reused.
    ///
    /// # Panics
    ///
    /// Panics if `path` is not valid UTF-8. Orrery import paths are always
    /// valid UTF-8 string literals, so this is not expected in practice.
    pub fn new(path: &Path) -> Self {
        let path_str = path.to_str().expect("file paths must be valid UTF-8");
        Self(interner::get_or_intern(path_str))
    }

    /// Resolves this [`FileId`] back to its path string.
    ///
    /// # Returns
    ///
    /// Returns a `&'static str` representing the original file path.
    pub fn as_str(self) -> &'static str {
        interner::resolve(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_returns_distinct_ids() {
        let a = FileId::new(Path::new("a.orr"));
        let b = FileId::new(Path::new("b.orr"));
        let c = FileId::new(Path::new("c.orr"));

        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn new_deduplicates_same_path() {
        let first = FileId::new(Path::new("shared/styles.orr"));
        let second = FileId::new(Path::new("shared/styles.orr"));

        assert_eq!(first, second);
    }

    #[test]
    fn as_str_returns_original_path() {
        let id = FileId::new(Path::new("shared/styles.orr"));
        assert_eq!(id.as_str(), "shared/styles.orr");
    }
}
