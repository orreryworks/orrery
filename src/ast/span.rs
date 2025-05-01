use std::fmt;

/// A generic wrapper for AST elements that tracks source position information.
///
/// `Spanned<T>` wraps any type `T` with location metadata, allowing parser and
/// elaboration code to provide rich diagnostic errors with precise source locations.
#[derive(Debug, Default)]
pub struct Spanned<T> {
    /// The wrapped value
    value: T,
    /// Starting offset in the source (byte position)
    offset: usize,
    /// Length of the spanned region in bytes
    length: usize,
}

impl<T> Spanned<T> {
    /// Create a new spanned value from a value and position information
    pub fn new(value: T, offset: usize, length: usize) -> Self {
        Self {
            value,
            offset,
            length,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn length(&self) -> usize {
        self.length
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
            offset: self.offset,
            length: self.length,
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
            offset: self.offset,
            length: self.length,
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
