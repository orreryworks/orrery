pub use chumsky::span::{SimpleSpan as SpanImpl, Span};
use std::fmt;

/// A generic wrapper for AST elements that tracks source position information.
///
/// `Spanned<T>` wraps any type `T` with location metadata, allowing parser and
/// elaboration code to provide rich diagnostic errors with precise source locations.
#[derive(Debug)]
pub struct Spanned<T> {
    /// The wrapped value
    value: T,
    /// The span information from the parser
    span: SpanImpl,
}

impl<T> Spanned<T> {
    /// Create a new spanned value from a value and span information
    pub fn new(value: T, span: SpanImpl) -> Self {
        Self { value, span }
    }

    pub fn offset(&self) -> usize {
        self.span.start
    }

    pub fn length(&self) -> usize {
        self.span.end - self.span.start
    }

    pub fn span(&self) -> SpanImpl {
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

    /// Clone the span information while discarding the inner value
    pub fn clone_spanned(&self) -> Spanned<()> {
        Spanned {
            value: (),
            span: self.span,
        }
    }
}

// Implement Default for Spanned<T> where T: Default
impl<T: Default> Default for Spanned<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            span: SpanImpl::new((), 0..0),
        }
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
