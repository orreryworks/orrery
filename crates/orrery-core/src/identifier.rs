//! Identifier management using string interning for efficient string storage and comparison.
//!
//! This module provides the [`Id`] type for efficient, namespace-aware identifier storage.
//! Each identifier is split into a name (final path segment) and an optional namespace
//! (everything before the last `::`), stored as separate interned symbols.
//!
//! Depends on the [`crate::interner`] module for global string storage and [`Symbol`] handles.

use std::fmt;

use crate::interner::{self, Symbol};

/// Efficient identifier using string interning.
///
/// Supports `::`-separated paths where the final segment is the name and everything
/// before it is the namespace. The unqualified name can be retrieved independently
/// from the full path.
///
/// # Examples
///
/// ```
/// # use orrery_core::identifier::Id;
/// let user_id = Id::new("user_service");
///
/// // Create anonymous identifiers
/// let anon_id = Id::from_anonymous();
///
/// // Create nested identifiers
/// let nested = user_id.create_nested(Id::new("database"));
/// assert_eq!(nested, "user_service::database");
///
/// // Access name and namespace separately
/// assert_eq!(nested.name(), "database");
/// assert_eq!(nested.namespace(), Some("user_service"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id {
    /// Final path segment (e.g., `"backend"` in `"system::backend"`).
    name: Symbol,
    /// Path prefix before the last `::`, if any (e.g., `"system"` in `"system::backend"`).
    namespace: Option<Symbol>,
}

impl Id {
    /// Creates an [`Id`] from &str.
    ///
    /// If `input` contains `::`, the last segment becomes the name and everything
    /// before it becomes the namespace. Otherwise the entire string is the name
    /// with no namespace.
    ///
    /// # Arguments
    ///
    /// * `input` - The string representation of the identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// # use orrery_core::identifier::Id;
    /// let simple = Id::new("Rectangle");
    /// assert_eq!(simple.name(), "Rectangle");
    /// assert_eq!(simple.namespace(), None);
    ///
    /// let namespaced = Id::new("system::frontend::app");
    /// assert_eq!(namespaced.name(), "app");
    /// assert_eq!(namespaced.namespace(), Some("system::frontend"));
    /// ```
    pub fn new(input: &str) -> Self {
        let mut interner = interner::interner();

        if let Some((ns_part, name_part)) = input.rsplit_once("::") {
            let ns = interner.get_or_intern(ns_part);
            let name = interner.get_or_intern(name_part);
            Self {
                name,
                namespace: Some(ns),
            }
        } else {
            let name = interner.get_or_intern(input);
            Self {
                name,
                namespace: None,
            }
        }
    }

    /// Creates an anonymous [`Id`] without string representation.
    ///
    /// Anonymous identifiers have no namespace.
    ///
    /// # Examples
    ///
    /// ```
    /// # use orrery_core::identifier::Id;
    /// let anon_id = Id::from_anonymous();
    /// ```
    pub fn from_anonymous() -> Self {
        let mut interner = interner::interner();
        let idx = interner.len();
        let anon_name = format!("__{idx}");
        let name_sym = interner.get_or_intern(&anon_name);
        Self {
            name: name_sym,
            namespace: None,
        }
    }

    /// Creates a nested [`Id`] by combining this identifier with a child.
    ///
    /// The parent's full path becomes the new namespace, and the child's name
    /// becomes the new name. If the child already has a namespace, it is
    /// incorporated into the resulting namespace.
    ///
    /// # Arguments
    ///
    /// * `child_id` - The child identifier to nest under this one.
    ///
    /// # Returns
    ///
    /// A new [`Id`] where `namespace` is the parent's full path and `name` is the child's name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use orrery_core::identifier::Id;
    /// let parent = Id::new("user");
    /// let child = Id::new("profile");
    /// let nested = parent.create_nested(child);
    /// assert_eq!(nested, "user::profile");
    /// assert_eq!(nested.name(), "profile");
    /// assert_eq!(nested.namespace(), Some("user"));
    /// ```
    pub fn create_nested(self, child_id: Id) -> Self {
        // Optimization: when parent has no namespace and child has no namespace,
        // the new namespace is simply the parent's name symbol — no allocation needed.
        if self.namespace.is_none() && child_id.namespace.is_none() {
            return Self {
                name: child_id.name,
                namespace: Some(self.name),
            };
        }

        let mut interner = interner::interner();

        let parent_full = self.full_path_with(&interner);

        // Append child's namespace (if any) to the parent's full path
        let new_namespace_str = match child_id.namespace {
            Some(child_ns) => {
                let child_ns_str = interner.resolve(child_ns);
                format!("{parent_full}::{child_ns_str}")
            }
            None => parent_full,
        };

        let new_namespace = interner.get_or_intern(&new_namespace_str);

        Self {
            name: child_id.name,
            namespace: Some(new_namespace),
        }
    }

    /// Returns the name (final path segment) of this identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// # use orrery_core::identifier::Id;
    /// let simple = Id::new("Rectangle");
    /// assert_eq!(simple.name(), "Rectangle");
    ///
    /// let nested = Id::new("system::backend");
    /// assert_eq!(nested.name(), "backend");
    /// ```
    pub fn name(&self) -> &str {
        interner::resolve(self.name)
    }

    /// Returns the namespace (everything before the final segment) of this identifier.
    ///
    /// # Returns
    ///
    /// `None` if this identifier has no namespace.
    ///
    /// # Examples
    ///
    /// ```
    /// # use orrery_core::identifier::Id;
    /// let simple = Id::new("Rectangle");
    /// assert_eq!(simple.namespace(), None);
    ///
    /// let nested = Id::new("system::backend");
    /// assert_eq!(nested.namespace(), Some("system"));
    /// ```
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.map(interner::resolve)
    }

    /// Resolves the full path (`{namespace}::{name}` or just `{name}`).
    fn full_path(&self) -> String {
        self.full_path_with(&interner::interner())
    }

    /// Resolves the full path using an already-acquired [`interner::Interner`] guard.
    fn full_path_with(self, interner: &interner::Interner) -> String {
        let name_str = interner.resolve(self.name);
        match self.namespace {
            Some(ns) => {
                let ns_str = interner.resolve(ns);
                format!("{ns_str}::{name_str}")
            }
            None => name_str.to_string(),
        }
    }
}

/// Formats the full path as `"{namespace}::{name}"`, or just `"{name}"` if no namespace.
impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full_path())
    }
}

/// Parses a string into an [`Id`].
///
/// Delegates to [`Id::new`].
impl std::str::FromStr for Id {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl From<&str> for Id {
    /// Creates an [`Id`] from a string slice.
    ///
    /// Delegates to [`Id::new`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use orrery_core::identifier::Id;
    /// let id: Id = "example".into();
    /// assert_eq!(id, "example");
    /// ```
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl PartialEq<str> for Id {
    /// Compares the full path (`{namespace}::{name}`) against a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use orrery_core::identifier::Id;
    /// let id = Id::new("Rectangle");
    /// assert!(id == "Rectangle");
    ///
    /// let nested = Id::new("system").create_nested(Id::new("backend"));
    /// assert!(nested == "system::backend");
    /// ```
    fn eq(&self, other: &str) -> bool {
        // Uses allocation-free slice comparison against the resolved symbols.
        let name;
        let ns;
        {
            let interner = interner::interner();
            name = interner.resolve(self.name);
            ns = self.namespace.map(|ns| interner.resolve(ns));
        }

        match ns {
            Some(ns) => {
                let expected_len = ns.len() + 2 + name.len();
                other.len() == expected_len
                    && other.starts_with(ns)
                    && other[ns.len()..].starts_with("::")
                    && other[ns.len() + 2..] == *name
            }
            None => other == name,
        }
    }
}

impl PartialEq<&str> for Id {
    /// Compares the full path against a `&str` reference.
    ///
    /// Delegates to [`PartialEq<str>`](Id#impl-PartialEq<str>-for-Id).
    ///
    /// # Examples
    ///
    /// ```
    /// # use orrery_core::identifier::Id;
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

        let namespaced = Id::new("system::backend");
        assert_eq!(namespaced.name(), "backend");
        assert_eq!(namespaced.namespace(), Some("system"));
        assert_eq!(namespaced, "system::backend");

        let deep = Id::new("a::b::c::d");
        assert_eq!(deep.name(), "d");
        assert_eq!(deep.namespace(), Some("a::b::c"));
        assert_eq!(deep, "a::b::c::d");
    }

    #[test]
    fn test_from_anonymous() {
        let id1 = Id::from_anonymous();
        let id2 = Id::from_anonymous();

        assert_ne!(id1, id2);
        assert_eq!(id1.namespace(), None);
        assert_eq!(id2.namespace(), None);
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

        let level1 = parent.create_nested(Id::new("frontend"));
        let level2 = level1.create_nested(Id::new("app"));
        let level3 = level2.create_nested(Id::new("component"));
        assert_eq!(level3, "system::frontend::app::component");

        let namespaced_child = parent.create_nested(Id::new("sub::component"));
        assert_eq!(namespaced_child, "system::sub::component");
        assert_eq!(namespaced_child.name(), "component");
        assert_eq!(namespaced_child.namespace(), Some("system::sub"));
    }

    #[test]
    fn test_name_and_namespace() {
        let simple = Id::new("Rectangle");
        assert_eq!(simple.name(), "Rectangle");
        assert_eq!(simple.namespace(), None);

        let nested = Id::new("system").create_nested(Id::new("backend"));
        assert_eq!(nested.name(), "backend");
        assert_eq!(nested.namespace(), Some("system"));

        let deep = Id::new("system")
            .create_nested(Id::new("frontend"))
            .create_nested(Id::new("app"))
            .create_nested(Id::new("component"));
        assert_eq!(deep.name(), "component");
        assert_eq!(deep.namespace(), Some("system::frontend::app"));
    }

    #[test]
    fn test_equivalence_nested_and_new() {
        let via_nested = Id::new("system").create_nested(Id::new("backend"));
        let via_new = Id::new("system::backend");
        assert_eq!(via_nested, via_new);
        assert_eq!(via_nested.name(), via_new.name());
        assert_eq!(via_nested.namespace(), via_new.namespace());

        let deep_nested = Id::new("a")
            .create_nested(Id::new("b"))
            .create_nested(Id::new("c"));
        let deep_new = Id::new("a::b::c");
        assert_eq!(deep_nested, deep_new);
        assert_eq!(deep_nested.name(), deep_new.name());
        assert_eq!(deep_nested.namespace(), deep_new.namespace());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Id::new("Rectangle")), "Rectangle");

        let nested = Id::new("parent").create_nested(Id::new("child"));
        assert_eq!(format!("{}", nested), "parent::child");

        let deep = Id::new("system")
            .create_nested(Id::new("frontend"))
            .create_nested(Id::new("app"));
        assert_eq!(format!("{}", deep), "system::frontend::app");
    }

    #[test]
    fn test_from_str() {
        let from_into: Id = "test_string".into();
        assert_eq!(from_into, Id::new("test_string"));

        let parsed: Id = "parent::child".parse().unwrap();
        assert_eq!(parsed.name(), "child");
        assert_eq!(parsed.namespace(), Some("parent"));
        assert_eq!(parsed, "parent::child");
    }

    #[test]
    fn test_partial_eq_str() {
        let id = Id::new("Rectangle");
        assert!(id == "Rectangle");
        assert!(id != "Oval");

        let nested = Id::new("parent::child");
        assert!(nested == "parent::child");
        assert!(nested != "parent");
        assert!(nested != "child");

        let empty = Id::new("");
        assert!(empty == "");
        assert!(empty != "non-empty");

        // PartialEq<&str>
        let name = String::from("Rectangle");
        assert!(id == name.as_str());
    }

    #[test]
    fn test_copy() {
        let id1 = Id::new("copy_test");
        let id2 = id1;
        let id3 = id1; // id1 should still be usable after id2 assignment

        assert_eq!(id1, id2);
        assert_eq!(id1, id3);
        assert_eq!(id1, "copy_test");
    }
}
