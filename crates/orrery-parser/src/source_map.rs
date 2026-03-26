//! Source map for multi-file span tracking.
//!
//! This module provides a virtual byte address space that maps spans from
//! multiple source files into a single continuous address space. Each file
//! is placed at a unique offset range, with 1-byte gaps between files to
//! ensure no span can accidentally straddle a file boundary.
//!
//! This follows the pattern used by `rustc` and `rust-analyzer`.
//!
//! # Virtual Address Space Layout
//!
//! ```text
//! File A (100 bytes): offsets 0..100
//! gap: 1 byte (offset 100)
//! File B (80 bytes):  offsets 101..181
//! gap: 1 byte (offset 181)
//! File C (50 bytes):  offsets 182..232
//! ```
//!
//! # Example
//!
//! ```ignore
//! use orrery_parser::source_map::SourceMap;
//!
//! let mut map = SourceMap::new();
//!
//! let offset_a = map.add_file("a.orr", "hello", None);
//! assert_eq!(offset_a, 0);
//!
//! let offset_b = map.add_file("b.orr", "world", None);
//! assert_eq!(offset_b, 6); // 5 bytes + 1-byte gap
//! ```

use crate::span::Span;

/// A single source file registered in the source map.
///
/// Each file occupies a contiguous range `[start_offset, end_offset)` in the
/// virtual address space. The `end_offset` is exclusive — valid byte positions
/// within this file are `start_offset..end_offset`.
#[derive(Debug, Clone)]
pub struct SourceFile<'a> {
    /// Human-readable name, typically a file path.
    name: String,
    /// The full source text of the file.
    source: &'a str,
    /// Byte offset where this file starts in the virtual address space.
    start_offset: usize,
    /// Byte offset where this file ends (exclusive) in the virtual address space.
    end_offset: usize,
    /// Span of the `import "path";` declaration that first imported this file.
    /// `None` for the root/entry file.
    first_imported_at: Option<Span>,
}

impl SourceFile<'_> {
    /// Returns the human-readable name of this file.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the source text of this file.
    pub fn source(&self) -> &str {
        self.source
    }

    /// Returns the start offset in the virtual address space.
    pub fn start_offset(&self) -> usize {
        self.start_offset
    }

    /// Returns the end offset (exclusive) in the virtual address space.
    pub fn end_offset(&self) -> usize {
        self.end_offset
    }

    /// Returns the span where this file was first imported, if any.
    ///
    /// Returns `None` for the root/entry file.
    pub fn first_imported_at(&self) -> Option<Span> {
        self.first_imported_at
    }

    /// Returns the length of the source text in bytes.
    pub fn len(&self) -> usize {
        self.source.len()
    }

    /// Returns `true` if the source text is empty.
    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }
}

/// Maps virtual byte offsets to source files.
///
/// `SourceMap` places all source files into a virtual byte address space,
/// enabling [`Span`] values from different files to coexist without
/// modification. Files are separated by 1-byte gaps to prevent spans
/// from accidentally straddling file boundaries.
///
/// # Example
///
/// ```ignore
/// use orrery_parser::source_map::SourceMap;
///
/// let mut map = SourceMap::new();
///
/// let base = map.add_file("main.orr", "diagram component;\nbox: Rectangle;", None);
///
/// let file = map.lookup_file(base).unwrap();
/// assert_eq!(file.name(), "main.orr");
/// ```
#[derive(Debug, Default)]
pub struct SourceMap<'a> {
    files: Vec<SourceFile<'a>>,
    /// Byte offset where the next added file will start.
    next_offset: usize,
}

impl<'a> SourceMap<'a> {
    /// Create a new, empty source map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a source file in the virtual address space.
    ///
    /// The file is placed at the next available offset, separated from the
    /// previous file by a 1-byte gap.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name for the file (typically its path).
    /// * `source` - The full source text.
    /// * `imported_at` - The span of the `import` declaration that triggered
    ///   loading this file, or `None` for the root file.
    ///
    /// # Returns
    ///
    /// The `base_offset` at which this file starts in the virtual address space.
    pub fn add_file(
        &mut self,
        name: impl Into<String>,
        source: &'a str,
        imported_at: Option<Span>,
    ) -> usize {
        let start_offset = self.next_offset;
        let end_offset = start_offset + source.len();

        self.files.push(SourceFile {
            name: name.into(),
            source,
            start_offset,
            end_offset,
            first_imported_at: imported_at,
        });

        // Next file starts after a 1-byte gap.
        self.next_offset = end_offset + 1;

        start_offset
    }

    /// Looks up the source file containing the given virtual offset.
    ///
    /// Uses binary search for O(log n) performance.
    ///
    /// # Arguments
    ///
    /// * `offset` - A byte offset in the virtual address space.
    ///
    /// # Returns
    ///
    /// The [`SourceFile`] containing `offset`, or `None` if the offset falls
    /// in a gap between files or is out of range.
    pub fn lookup_file(&self, offset: usize) -> Option<&SourceFile<'a>> {
        // `partition_point` returns the count of files whose `start_offset <= offset`.
        let idx = self.files.partition_point(|f| f.start_offset <= offset);
        if idx == 0 {
            return None;
        }
        let file = &self.files[idx - 1];
        if offset < file.end_offset {
            Some(file)
        } else {
            None // offset is in a gap or past the last file
        }
    }

    /// Extracts the source text corresponding to a span.
    ///
    /// # Arguments
    ///
    /// * `span` - A [`Span`] in the virtual address space.
    ///
    /// # Returns
    ///
    /// The source text slice, or `None` if the span crosses a file boundary,
    /// falls in a gap, or is out of range.
    pub fn source_slice(&self, span: Span) -> Option<&str> {
        let file = self.lookup_file(span.start())?;
        // Verify the entire span stays within this file.
        if span.end() > file.end_offset {
            return None;
        }
        let local_start = span.start() - file.start_offset;
        let local_end = span.end() - file.start_offset;
        Some(&file.source[local_start..local_end])
    }

    /// Returns the number of registered source files.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Returns all registered source files.
    pub fn files(&self) -> &[SourceFile<'a>] {
        &self.files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_single_file_returns_zero_offset() {
        let mut map = SourceMap::new();
        let offset = map.add_file("a.orr", "hello", None);
        assert_eq!(offset, 0);
        assert_eq!(map.file_count(), 1);
    }

    #[test]
    fn add_multiple_files_with_gaps() {
        let mut map = SourceMap::new();

        // File A: 5 bytes at 0..5
        let a = map.add_file("a.orr", "hello", None);
        assert_eq!(a, 0);

        // File B: 5 bytes at 6..11 (1-byte gap at offset 5)
        let b = map.add_file("b.orr", "world", None);
        assert_eq!(b, 6);

        // File C: 3 bytes at 12..15 (1-byte gap at offset 11)
        let c = map.add_file("c.orr", "foo", None);
        assert_eq!(c, 12);

        assert_eq!(map.file_count(), 3);
    }

    #[test]
    fn add_empty_file() {
        let mut map = SourceMap::new();

        let a = map.add_file("empty.orr", "", None);
        assert_eq!(a, 0);

        // Next file starts at offset 1 (0-byte file + 1-byte gap).
        let b = map.add_file("b.orr", "x", None);
        assert_eq!(b, 1);
    }

    #[test]
    fn add_file_records_imported_at_span() {
        let mut map = SourceMap::new();
        let import_span = Span::new(10..25);

        map.add_file("root.orr", "root", None);
        map.add_file("lib.orr", "lib", Some(import_span));

        assert!(map.files()[0].first_imported_at().is_none());
        assert_eq!(map.files()[1].first_imported_at(), Some(import_span));
    }

    #[test]
    fn source_file_accessors() {
        let mut map = SourceMap::new();
        map.add_file("test.orr", "abc", None);

        let file = &map.files()[0];
        assert_eq!(file.name(), "test.orr");
        assert_eq!(file.source(), "abc");
        assert_eq!(file.start_offset(), 0);
        assert_eq!(file.end_offset(), 3);
        assert_eq!(file.len(), 3);
        assert!(!file.is_empty());
    }

    #[test]
    fn empty_source_file() {
        let mut map = SourceMap::new();
        map.add_file("empty.orr", "", None);

        let file = &map.files()[0];
        assert_eq!(file.len(), 0);
        assert!(file.is_empty());
        assert_eq!(file.start_offset(), file.end_offset());
    }

    #[test]
    fn lookup_file_single() {
        let mut map = SourceMap::new();
        map.add_file("a.orr", "hello", None);

        // Every byte position within the file resolves.
        for i in 0..5 {
            let file = map.lookup_file(i);
            assert!(file.is_some(), "offset {i} should resolve");
            assert_eq!(file.unwrap().name(), "a.orr");
        }

        // Past end → None.
        assert!(map.lookup_file(5).is_none());
        assert!(map.lookup_file(100).is_none());
    }

    #[test]
    fn lookup_file_multiple() {
        let mut map = SourceMap::new();
        map.add_file("a.orr", "hello", None); // 0..5
        map.add_file("b.orr", "world", None); // 6..11
        map.add_file("c.orr", "foo", None); // 12..15

        assert_eq!(map.lookup_file(0).unwrap().name(), "a.orr");
        assert_eq!(map.lookup_file(4).unwrap().name(), "a.orr");
        assert!(map.lookup_file(5).is_none()); // gap
        assert_eq!(map.lookup_file(6).unwrap().name(), "b.orr");
        assert_eq!(map.lookup_file(10).unwrap().name(), "b.orr");
        assert!(map.lookup_file(11).is_none()); // gap
        assert_eq!(map.lookup_file(12).unwrap().name(), "c.orr");
        assert_eq!(map.lookup_file(14).unwrap().name(), "c.orr");
        assert!(map.lookup_file(15).is_none()); // past end
    }

    #[test]
    fn lookup_file_empty_map() {
        let map = SourceMap::new();
        assert!(map.lookup_file(0).is_none());
    }

    #[test]
    fn lookup_file_empty_file_returns_none() {
        let mut map = SourceMap::new();
        map.add_file("empty.orr", "", None); // 0..0

        // Empty file has no valid byte positions.
        assert!(map.lookup_file(0).is_none());
    }

    #[test]
    fn source_slice_within_file() {
        let mut map = SourceMap::new();
        map.add_file("a.orr", "hello world", None);

        let slice = map.source_slice(Span::new(0..5)).unwrap();
        assert_eq!(slice, "hello");

        let slice = map.source_slice(Span::new(6..11)).unwrap();
        assert_eq!(slice, "world");
    }

    #[test]
    fn source_slice_entire_file() {
        let mut map = SourceMap::new();
        map.add_file("a.orr", "hello", None);

        let slice = map.source_slice(Span::new(0..5)).unwrap();
        assert_eq!(slice, "hello");
    }

    #[test]
    fn source_slice_empty_span() {
        let mut map = SourceMap::new();
        map.add_file("a.orr", "hello", None);

        let slice = map.source_slice(Span::new(2..2)).unwrap();
        assert_eq!(slice, "");
    }

    #[test]
    fn source_slice_second_file() {
        let mut map = SourceMap::new();
        map.add_file("a.orr", "hello", None); // 0..5
        map.add_file("b.orr", "world", None); // 6..11

        let slice = map.source_slice(Span::new(6..11)).unwrap();
        assert_eq!(slice, "world");

        let slice = map.source_slice(Span::new(8..11)).unwrap();
        assert_eq!(slice, "rld");
    }

    #[test]
    fn source_slice_crossing_files_returns_none() {
        let mut map = SourceMap::new();
        map.add_file("a.orr", "hello", None); // 0..5
        map.add_file("b.orr", "world", None); // 6..11

        // Span from file A into the gap.
        assert!(map.source_slice(Span::new(3..6)).is_none());

        // Span from file A into file B.
        assert!(map.source_slice(Span::new(3..8)).is_none());
    }

    #[test]
    fn source_slice_in_gap_returns_none() {
        let mut map = SourceMap::new();
        map.add_file("a.orr", "hello", None); // 0..5
        map.add_file("b.orr", "world", None); // 6..11

        // Span starting in the gap.
        assert!(map.source_slice(Span::new(5..6)).is_none());
    }

    #[test]
    fn source_slice_out_of_range_returns_none() {
        let mut map = SourceMap::new();
        map.add_file("a.orr", "hello", None);

        assert!(map.source_slice(Span::new(100..105)).is_none());
    }

    #[test]
    fn virtual_address_space_layout() {
        let source_a = "x".repeat(100);
        let source_b = "y".repeat(80);
        let source_c = "z".repeat(50);
        let mut map = SourceMap::new();

        // File A: 100 bytes → offsets 0..100
        let a = map.add_file("a.orr", &source_a, None);
        assert_eq!(a, 0);
        assert_eq!(map.files()[0].start_offset(), 0);
        assert_eq!(map.files()[0].end_offset(), 100);

        // File B: 80 bytes → offsets 101..181 (gap at 100)
        let b = map.add_file("b.orr", &source_b, None);
        assert_eq!(b, 101);
        assert_eq!(map.files()[1].start_offset(), 101);
        assert_eq!(map.files()[1].end_offset(), 181);

        // File C: 50 bytes → offsets 182..232 (gap at 181)
        let c = map.add_file("c.orr", &source_c, None);
        assert_eq!(c, 182);
        assert_eq!(map.files()[2].start_offset(), 182);
        assert_eq!(map.files()[2].end_offset(), 232);

        // Gap bytes resolve to no file.
        assert!(map.lookup_file(100).is_none());
        assert!(map.lookup_file(181).is_none());

        // Last valid byte of each file resolves correctly.
        assert_eq!(map.lookup_file(99).unwrap().name(), "a.orr");
        assert_eq!(map.lookup_file(180).unwrap().name(), "b.orr");
        assert_eq!(map.lookup_file(231).unwrap().name(), "c.orr");

        // Past end is out of range.
        assert!(map.lookup_file(232).is_none());
    }
}
