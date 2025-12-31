use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    /// Create a new span from a range
    pub fn new(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }

    /// Get the start offset of the span
    pub fn start(&self) -> usize {
        self.start
    }

    /// Get the end offset of the span
    pub fn end(&self) -> usize {
        self.end
    }

    /// Get the length of the span
    pub fn len(&self) -> usize {
        if self.end >= self.start {
            self.end - self.start
        } else {
            panic!("Invalid span: end offset is less than start offset");
        }
    }

    /// Check if the span is empty
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Create a union of two spans (encompassing both)
    pub fn union(&self, other: Span) -> Span {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::new(0..0)
    }
}

impl From<Span> for miette::SourceSpan {
    fn from(span: Span) -> Self {
        miette::SourceSpan::new(span.start().into(), span.len())
    }
}

impl From<&Span> for miette::SourceSpan {
    fn from(span: &Span) -> Self {
        miette::SourceSpan::new(span.start().into(), span.len())
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(range: std::ops::Range<usize>) -> Self {
        Self::new(range)
    }
}

/// A generic wrapper for AST elements that tracks source position information.
///
/// `Spanned<T>` wraps any type `T` with location metadata, allowing parser and
/// elaboration code to provide rich diagnostic errors with precise source locations.
#[derive(Debug, Default, Clone)]
pub struct Spanned<T> {
    /// The wrapped value
    value: T,
    /// The span information from the parser
    span: Span,
}

impl<T> Spanned<T> {
    /// Create a new spanned value from a value and span information
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub fn offset(&self) -> usize {
        self.span.start()
    }

    pub fn length(&self) -> usize {
        self.span.len()
    }

    pub fn span(&self) -> Span {
        self.span
    }

    /// Convert from one spanned type to another using the provided function
    ///
    /// This maintains the same span information while transforming the value.
    pub fn map<F, U>(&self, f: F) -> Spanned<U>
    where
        F: FnOnce(&T) -> U,
    {
        Spanned {
            value: f(&self.value),
            span: self.span,
        }
    }

    /// Get a reference to the underlying value
    pub fn inner(&self) -> &T {
        &self.value
    }

    /// Consume the Spanned wrapper and return just the inner value
    pub fn into_inner(self) -> T {
        self.value
    }
}

// Implement Deref to make Spanned<T> easier to use
impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

// Implement Display by delegating to the inner value's Display implementation
impl<T: fmt::Display> fmt::Display for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

// PartialEq compares only the inner values, ignoring span information
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
    fn test_miette_conversion() {
        let span = Span::new(5..10);
        let miette_span: miette::SourceSpan = span.into();
        assert_eq!(miette_span.offset(), 5);
        assert_eq!(miette_span.len(), 5);
    }

    #[test]
    fn test_miette_conversion_by_ref() {
        let span = Span::new(5..10);
        let miette_span: miette::SourceSpan = (&span).into();
        assert_eq!(miette_span.offset(), 5);
        assert_eq!(miette_span.len(), 5);
    }

    #[test]
    fn test_spanned_with_new_span() {
        let span = Span::new(5..10);
        let spanned = Spanned::new("test", span);
        assert_eq!(spanned.offset(), 5);
        assert_eq!(spanned.length(), 5);
        assert_eq!(*spanned.inner(), "test");
    }
}
