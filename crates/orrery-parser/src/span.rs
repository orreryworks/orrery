//! Source code span tracking.
//!
//! This module provides types for tracking positions within source text,
//! enabling precise source location information for diagnostics, suggestions,
//! logging, and other tools that reference source code.
//!
//! - [`Span`] - A byte range within source text.
//! - [`Spanned<T>`] - A value with associated source location.
//!
//! # Example
//!
//! ```
//! # use orrery_parser::Span;
//! let span = Span::new(10..25);
//! assert_eq!(span.start(), 10);
//! assert_eq!(span.end(), 25);
//! assert_eq!(span.len(), 15);
//! ```

use std::{fmt, ops::Deref};

/// A byte range representing a location in source text.
///
/// Spans track the start and end byte offsets of syntax elements,
/// enabling precise source mapping for diagnostics, suggestions, and tooling.
///
/// # Examples
///
/// ```
/// # use orrery_parser::Span;
/// // Create a span covering bytes 10-25
/// let span = Span::new(10..25);
/// assert_eq!(span.start(), 10);
/// assert_eq!(span.end(), 25);
/// assert_eq!(span.len(), 15);
///
/// // Combine two spans into one that covers both
/// let other = Span::new(30..40);
/// let combined = span.union(other);
/// assert_eq!(combined.start(), 10);
/// assert_eq!(combined.end(), 40);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    /// Creates a new [`Span`] from a byte range.
    ///
    /// # Arguments
    ///
    /// * `range` - The start..end byte offset range.
    pub fn new(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }

    /// Creates a zero-length, empty span.
    pub fn empty() -> Self {
        Self::new(0..0)
    }

    /// Returns the start byte offset.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the end byte offset.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the length of the span in bytes.
    ///
    /// # Panics
    ///
    /// Panics if `end` is less than `start`.
    pub fn len(&self) -> usize {
        if self.end >= self.start {
            self.end - self.start
        } else {
            panic!("Invalid span: end offset is less than start offset");
        }
    }

    /// Returns `true` if the span has zero length.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Creates the smallest span encompassing both `self` and `other`.
    ///
    /// If either span is empty, the other is returned unchanged.
    ///
    /// # Arguments
    ///
    /// * `other` - The span to merge with.
    pub fn union(&self, other: Span) -> Span {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return *self;
        }
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Returns a new span with both offsets translated by `base` bytes.
    ///
    /// This shifts the span's position in the virtual address space
    /// without changing its length.
    ///
    /// # Arguments
    ///
    /// * `base` - Number of bytes to add to both `start` and `end`.
    pub fn shift(self, base: usize) -> Span {
        Self {
            start: self.start + base,
            end: self.end + base,
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(range: std::ops::Range<usize>) -> Self {
        Self::new(range)
    }
}

/// A value paired with its source location in the input text.
///
/// `Spanned<T>` wraps any type `T` with a [`Span`], carrying precise source
/// locations through to diagnostics.
///
/// Equality comparison ([`PartialEq`]) considers only the inner value;
/// two `Spanned` values with different spans but equal inner values are equal.
///
/// # Examples
///
/// ```ignore
/// let name = Spanned::new("server", Span::new(4..10));
/// assert_eq!(*name.inner(), "server");
/// assert_eq!(name.span().start(), 4);
///
/// // Deref lets you call methods on the inner value directly
/// assert!(name.starts_with("ser"));
/// ```
#[derive(Debug, Default, Clone, Eq)]
pub struct Spanned<T> {
    /// The wrapped value.
    value: T,
    /// Source location of this value.
    span: Span,
}

impl<T> Spanned<T> {
    /// Creates a new [`Spanned`] value.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to wrap.
    /// * `span` - The source location of `value`.
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    /// Returns the source [`Span`] associated with this value.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Transforms the inner value while preserving the span.
    ///
    /// # Arguments
    ///
    /// * `f` - A function applied to a reference of the inner value.
    pub fn map<F, U>(&self, f: F) -> Spanned<U>
    where
        F: FnOnce(&T) -> U,
    {
        Spanned {
            value: f(&self.value),
            span: self.span,
        }
    }

    /// Returns a reference to the inner value.
    pub fn inner(&self) -> &T {
        &self.value
    }

    /// Consumes the wrapper and returns the inner value.
    #[allow(dead_code)]
    pub fn into_inner(self) -> T {
        self.value
    }
}

// Delegates field/method access to the inner value.
impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

// Delegates formatting to the inner value.
impl<T: fmt::Display> fmt::Display for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

// Compares only the inner values, ignoring span information.
impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_basic_functionality() {
        let span = Span::new(5..10);
        assert_eq!(span.start(), 5);
        assert_eq!(span.end(), 10);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_span_empty() {
        let span = Span::new(5..5);
        assert_eq!(span.len(), 0);
        assert!(span.is_empty());
    }

    #[test]
    fn test_span_union() {
        let span1 = Span::new(5..10);
        let span2 = Span::new(15..20);
        let union = span1.union(span2);
        assert_eq!(union.start(), 5);
        assert_eq!(union.end(), 20);
    }

    #[test]
    fn test_span_shift() {
        let span = Span::new(5..10);
        let shifted = span.shift(100);
        assert_eq!(shifted.start(), 105);
        assert_eq!(shifted.end(), 110);
        assert_eq!(shifted.len(), 5);
    }

    #[test]
    fn test_span_shift_zero_is_identity() {
        let span = Span::new(5..10);
        assert_eq!(span.shift(0), span);
    }

    #[test]
    fn test_spanned_with_new_span() {
        let span = Span::new(5..10);
        let spanned = Spanned::new("test", span);
        assert_eq!(spanned.span().start(), 5);
        assert_eq!(spanned.span().len(), 5);
        assert_eq!(*spanned.inner(), "test");
    }

    #[test]
    fn test_span_empty_constructor() {
        let span = Span::empty();
        assert_eq!(span.start(), 0);
        assert_eq!(span.end(), 0);
        assert_eq!(span.len(), 0);
        assert!(span.is_empty());
    }

    #[test]
    fn test_span_default_is_empty() {
        let span = Span::default();
        assert_eq!(span, Span::empty());
        assert!(span.is_empty());
    }

    #[test]
    fn test_span_union_with_left_empty() {
        let empty = Span::empty();
        let span = Span::new(5..10);
        let union = empty.union(span);
        assert_eq!(union, span);
    }

    #[test]
    fn test_span_union_with_right_empty() {
        let span = Span::new(5..10);
        let empty = Span::empty();
        let union = span.union(empty);
        assert_eq!(union, span);
    }

    #[test]
    fn test_span_union_both_empty() {
        let union = Span::empty().union(Span::empty());
        assert!(union.is_empty());
    }

    #[test]
    fn test_spanned_eq_ignores_span() {
        let a = Spanned::new(42, Span::new(0..5));
        let b = Spanned::new(42, Span::new(10..20));
        assert_eq!(a, b);
    }

    #[test]
    fn test_spanned_ne_different_values() {
        let a = Spanned::new(1, Span::new(0..5));
        let b = Spanned::new(2, Span::new(0..5));
        assert_ne!(a, b);
    }

    #[test]
    fn test_spanned_map() {
        let spanned = Spanned::new(5, Span::new(0..10));
        let mapped = spanned.map(|v| v * 2);
        assert_eq!(*mapped.inner(), 10);
        assert_eq!(mapped.span(), spanned.span());
    }

    #[test]
    fn test_spanned_into_inner() {
        let spanned = Spanned::new(String::from("hello"), Span::new(0..5));
        let value = spanned.into_inner();
        assert_eq!(value, "hello");
    }

    #[test]
    fn test_spanned_deref() {
        let spanned = Spanned::new(String::from("hello"), Span::new(0..5));
        // Deref lets us call String methods directly
        assert_eq!(spanned.len(), 5);
        assert!(spanned.starts_with("he"));
    }

    #[test]
    fn test_spanned_display() {
        let spanned = Spanned::new(42, Span::new(0..5));
        assert_eq!(format!("{spanned}"), "42");
    }
}
