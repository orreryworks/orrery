//! Shared global string interner.
//!
//! Provides a single [`StringInterner`] that stores all interned strings
//! for the lifetime of the program.
//!
//! # Usage
//!
//! For single operations, use the free functions [`get_or_intern`] and
//! [`resolve`]. When multiple operations should share a single lock
//! acquisition, call [`interner`] to obtain a [`MutexGuard`] over the
//! global [`Interner`] and use its methods directly.
//!
//! # Thread Safety
//!
//! Access is serialized through a [`Mutex`]. The free functions each acquire
//! and release the lock per call. [`interner`] returns a [`MutexGuard`]
//! that holds the lock until dropped.
//!
//! # Safety
//!
//! [`Interner::resolve`] (and the free [`resolve`] function) return
//! `&'static str` via an `unsafe` lifetime extension. This is sound because:
//! 1. The interner lives in a `static` [`OnceLock`] — it is never dropped.
//! 2. [`BucketBackend`] allocates strings in fixed buckets that are never
//!    moved or deallocated.
//! 3. Strings are never removed from the interner once interned.

use std::sync::{Mutex, MutexGuard, OnceLock};

use string_interner::{DefaultSymbol, StringInterner, backend::BucketBackend};

type Inner = StringInterner<BucketBackend>;

/// Global string interner instance.
static INTERNER: OnceLock<Mutex<Interner>> = OnceLock::new();

/// Opaque handle to an interned string.
///
/// A lightweight token that represents a previously interned string.
/// Resolve it back to a `&'static str` via [`Interner::resolve`] or the
/// free [`resolve`] function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(DefaultSymbol);

/// Shared global string interner.
///
/// # Examples
///
/// ```
/// # use orrery_core::interner::interner;
/// let mut inter = interner();
/// let sym = inter.get_or_intern("hello");
/// assert_eq!(inter.resolve(sym), "hello");
/// ```
pub struct Interner(Inner);

impl Interner {
    /// Interns a string, returning a cheap [`Symbol`] handle.
    ///
    /// If the string was already interned, the existing symbol is returned.
    pub fn get_or_intern(&mut self, s: &str) -> Symbol {
        Symbol(self.0.get_or_intern(s))
    }

    /// Returns the number of strings currently held by the interner.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Resolves a [`Symbol`] to a `&'static str`.
    ///
    /// # Panics
    ///
    /// Panics if `symbol` was not produced by this interner.
    pub fn resolve(&self, symbol: Symbol) -> &'static str {
        let s = self
            .0
            .resolve(symbol.0)
            .expect("symbol should exist in interner");
        // SAFETY: See module-level safety documentation.
        unsafe { &*(s as *const str) }
    }
}

/// Acquires the global interner lock.
///
/// Returns a [`MutexGuard`]. Use this when you need multiple
/// operations under a single lock acquisition. For one-shot
/// calls, prefer the free functions [`get_or_intern`] and [`resolve`].
pub fn interner() -> MutexGuard<'static, Interner> {
    INTERNER
        .get_or_init(|| Mutex::new(Interner(Inner::new())))
        .lock()
        .expect("interner lock poisoned")
}

/// Interns a string, returning a cheap [`Symbol`] handle.
///
/// Convenience wrapper that acquires and releases the lock for a single
/// operation.
pub fn get_or_intern(s: &str) -> Symbol {
    interner().get_or_intern(s)
}

/// Resolves a [`Symbol`] to a `&'static str`.
///
/// Convenience wrapper that acquires and releases the lock for a single
/// operation.
///
/// # Panics
///
/// Panics if the interner lock is poisoned or if `symbol` was not produced
/// by this interner.
pub fn resolve(symbol: Symbol) -> &'static str {
    interner().resolve(symbol)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn round_trip_returns_original_string() {
        let sym = get_or_intern("interner_test_round_trip");
        assert_eq!(resolve(sym), "interner_test_round_trip");
    }

    #[test]
    fn deduplication_returns_same_symbol() {
        let first = get_or_intern("interner_test_dedup");
        let second = get_or_intern("interner_test_dedup");
        assert_eq!(first, second);
    }

    #[test]
    fn distinct_strings_yield_distinct_symbols() {
        let a = get_or_intern("interner_test_distinct_a");
        let b = get_or_intern("interner_test_distinct_b");
        assert_ne!(a, b);
    }

    #[test]
    fn static_str_survives_after_lock_dropped() {
        let sym = get_or_intern("interner_test_static_lifetime");

        // Resolve inside a locked scope, then drop the guard.
        let s: &'static str = {
            let guard = interner();
            guard.resolve(sym)
        };
        // The guard is dropped here, but `s` must still be valid.
        assert_eq!(s, "interner_test_static_lifetime");
    }

    #[test]
    fn len_increases_after_interning_new_string() {
        let mut guard = interner();
        let before = guard.len();
        guard.get_or_intern("interner_test_len_unique_string");
        assert_eq!(guard.len(), before + 1);
    }

    #[test]
    fn len_stable_after_interning_duplicate() {
        let mut guard = interner();
        guard.get_or_intern("interner_test_len_dup");
        let after_first = guard.len();
        guard.get_or_intern("interner_test_len_dup");
        assert_eq!(guard.len(), after_first);
    }
}
