//! Identifier management using string interning for efficient string storage and comparison
//!
//! This module provides the [`Id`] type with an efficient string-interner based approach.

use std::{
    fmt,
    sync::{Mutex, OnceLock},
};

use string_interner::{DefaultStringInterner, DefaultSymbol};

/// Global string interner for efficient identifier storage.
///
/// # Thread Safety
///
/// This uses `Mutex` for thread-safe access to the string interner.
static INTERNER: OnceLock<Mutex<DefaultStringInterner>> = OnceLock::new();

/// Efficient identifier type using string interning
///
/// This type provides efficient storage and comparison of string identifiers through
/// string interning.
///
/// # Examples
///
/// ```
/// use filament_core::identifier::Id;
///
/// // Create identifiers from names
/// let rect_id = Id::new("Rectangle");
/// let user_id = Id::new("user_service");
///
/// // Create anonymous identifiers
/// let anon_id = Id::from_anonymous(0);
///
/// // Create nested identifiers
/// let nested_id = user_id.create_nested(Id::new("database"));
/// assert_eq!(nested_id, "user_service::database");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(DefaultSymbol);

impl Id {
    /// Creates an `Id` from &str.
    ///
    /// # Arguments
    ///
    /// * `name` - The string representation of the identifier
    ///
    /// # Examples
    ///
    /// ```
    /// use filament_core::identifier::Id;
    ///
    /// let component_id = Id::new("user_service");
    /// let type_id = Id::new("Rectangle");
    /// ```
    pub fn new(name: &str) -> Self {
        let mut interner = INTERNER
            .get_or_init(|| Mutex::new(DefaultStringInterner::new()))
            .lock()
            .expect("Failed to acquire interner lock");
        let symbol = interner.get_or_intern(name);
        Self(symbol)
    }

    /// Creates an internal `Id` identifier without string representation.
    ///
    /// # Arguments
    ///
    /// * `idx` - A unique index used to generate the anonymous identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use filament_core::identifier::Id;
    ///
    /// let anon_id = Id::from_anonymous(42);
    /// ```
    pub fn from_anonymous(idx: usize) -> Self {
        let name = format!("__{idx}");
        Self::new(&name)
    }

    /// Creates a nested ID by combining parent ID and child ID with '::' separator.
    ///
    /// # Arguments
    ///
    /// * `child_id` - The child identifier to append.
    ///
    /// # Examples
    ///
    /// ```
    /// use filament_core::identifier::Id;
    ///
    /// let parent = Id::new("user");
    /// let child = Id::new("profile");
    /// let nested = parent.create_nested(child);
    /// assert_eq!(nested, "user::profile");
    /// ```
    pub fn create_nested(&self, child_id: Id) -> Self {
        let mut interner = INTERNER
            .get_or_init(|| Mutex::new(DefaultStringInterner::new()))
            .lock()
            .expect("Failed to acquire interner lock");
        let parent_str = interner
            .resolve(self.0)
            .expect("Parent ID should exist in interner");
        let child_str = interner
            .resolve(child_id.0)
            .expect("Child ID should exist in interner");
        let nested_name = format!("{}::{}", parent_str, child_str);
        let symbol = interner.get_or_intern(&nested_name);
        Self(symbol)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let interner = INTERNER
            .get_or_init(|| Mutex::new(DefaultStringInterner::new()))
            .lock()
            .expect("Failed to acquire interner lock");
        let str_value = interner
            .resolve(self.0)
            .expect("Symbol should exist in interner");
        write!(f, "{}", str_value)
    }
}

impl std::str::FromStr for Id {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut interner = INTERNER
            .get_or_init(|| Mutex::new(DefaultStringInterner::new()))
            .lock()
            .expect("Failed to acquire interner lock");
        let symbol = interner.get_or_intern(s);
        Ok(Self(symbol))
    }
}

impl From<&str> for Id {
    /// Creates an `Id` from a string slice
    ///
    /// This is a convenience implementation that calls `Id::new`.
    ///
    /// # Examples
    ///
    /// ```
    /// use filament_core::identifier::Id;
    ///
    /// let id: Id = "example".into();
    /// assert_eq!(id, "example");
    /// ```
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl PartialEq<str> for Id {
    /// Allows direct comparison with string slices: `id == "string"`
    ///
    /// # Examples
    ///
    /// ```
    /// use filament_core::identifier::Id;
    ///
    /// let id = Id::new("Rectangle");
    /// assert!(id == "Rectangle");
    /// ```
    fn eq(&self, other: &str) -> bool {
        let interner = INTERNER
            .get_or_init(|| Mutex::new(DefaultStringInterner::new()))
            .lock()
            .expect("Failed to acquire interner lock");
        let self_str = interner
            .resolve(self.0)
            .expect("Symbol should exist in interner");
        self_str == other
    }
}

impl PartialEq<&str> for Id {
    /// Allows direct comparison with string references: `id == &string`
    ///
    /// # Examples
    ///
    /// ```
    /// use filament_core::identifier::Id;
    ///
    /// let id = Id::new("Rectangle");
    /// let name = "Rectangle";
    /// assert!(id == name);
    /// ```
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let id1 = Id::new("Rectangle");
        let id2 = Id::new("Rectangle");
        let id3 = Id::new("Oval");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_eq!(id1, "Rectangle");
    }

    #[test]
    fn test_from_anonymous() {
        let id1 = Id::from_anonymous(0);
        let id2 = Id::from_anonymous(1);
        let id3 = Id::from_anonymous(0);

        assert_ne!(id1, id2);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_create_nested() {
        let parent = Id::new("system");
        let child1 = Id::new("backend");
        let child2 = Id::new("frontend");

        let nested1 = parent.create_nested(child1);
        let nested2 = parent.create_nested(child2);

        assert_ne!(nested1, nested2);
        assert_eq!(nested1, "system::backend");
        assert_eq!(nested2, "system::frontend");
    }

    #[test]
    fn test_deep_nesting() {
        let root = Id::new("system");
        let frontend = Id::new("frontend");
        let app = Id::new("app");
        let component = Id::new("component");

        let level1 = root.create_nested(frontend);
        let level2 = level1.create_nested(app);
        let level3 = level2.create_nested(component);

        assert_eq!(level3, "system::frontend::app::component");
    }

    #[test]
    fn test_to_string() {
        let id = Id::new("test_component");
        assert_eq!(id, "test_component");
    }

    #[test]
    fn test_display_trait() {
        let id = Id::new("display_test");
        assert_eq!(format!("{}", id), "display_test");
    }

    #[test]
    fn test_from_trait() {
        let id1: Id = "test_string".into();
        let id2 = Id::new("test_string");

        assert_eq!(id1, id2);
        assert_eq!(id1, "test_string");
    }

    #[test]
    fn test_hash_and_eq() {
        use std::collections::HashMap;

        let id1 = Id::new("key1");
        let id2 = Id::new("key1");
        let id3 = Id::new("key2");

        let mut map = HashMap::new();
        map.insert(id1, "value1");
        map.insert(id3, "value2");

        assert_eq!(map.get(&id2), Some(&"value1"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_copy_trait() {
        let id1 = Id::new("copy_test");
        let id2 = id1; // This should work because Id implements Copy
        let id3 = id1; // id1 should still be usable after id2 assignment

        // All three should be equal and id1 should still be usable
        assert_eq!(id1, id2);
        assert_eq!(id1, id3);
        assert_eq!(id2, id3);
        assert_eq!(id1, "copy_test");
        assert_eq!(id2, "copy_test");
        assert_eq!(id3, "copy_test");
    }

    #[test]
    fn test_partial_eq_str() {
        // Test PartialEq<str> implementation
        let id = Id::new("Rectangle");

        // Test equality with str literal
        assert!(id == "Rectangle");

        // Test inequality with str literal
        assert!(id != "Oval");

        // Test with nested identifiers
        let nested = Id::new("parent::child");
        assert!(nested == "parent::child");
        assert!(nested != "parent");
        assert!(nested != "child");

        // Test with empty string
        let empty = Id::new("");
        assert!(empty == "");
        assert!(empty != "non-empty");
    }

    #[test]
    fn test_partial_eq_str_ref() {
        // Test PartialEq<&str> implementation
        let id = Id::new("Component");

        let name1 = String::from("Component");
        let name2 = String::from("Element");

        // Test equality with &str reference
        assert!(id == name1.as_str());

        // Test inequality with &str reference
        assert!(id != name2.as_str());

        // Test with borrowed string slices
        let slice: &str = "Component";
        assert!(id == slice);

        // Test with nested identifiers
        let nested = Id::new("frontend::app");
        let nested_str = String::from("frontend::app");
        assert!(nested == nested_str.as_str());
    }
}
