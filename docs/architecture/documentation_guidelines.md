# Orrery Documentation Guidelines

## Core Principles

1. **Every public item MUST be documented** - No exceptions for mature crates
2. **Explain WHY, not just WHAT** - Describe purpose and behavior, not how it's used elsewhere
3. **Document dependencies, not dependents** - Reference upstream dependencies, never mention downstream consumers
4. **Consistency is paramount** - Same patterns across all crates
5. **Examples for complex APIs** - Show, don't just tell
6. **Private code needs context** - Even internal modules should explain their purpose

---

## Documentation Syntax

| Syntax | Purpose | Location |
|--------|---------|----------|
| `//!` | Module/crate-level docs | Top of file |
| `///` | Item-level docs | Above items |
| `//` | Implementation comments | Within code |

---

## Requirements by Item Type

### Public Items

| Item | Summary | Details | Examples | Sections |
|------|---------|---------|----------|----------|
| Module | Required | Required | Recommended | N/A |
| Struct/Enum | Required | Required | Required* | N/A |
| Trait | Required | Required | Required* | N/A |
| Trait Methods | Required | Required | Optional | Required |
| Function | Required | Required | Required* | Required |
| Method | Required | Required | Optional | Required |

*Required for complex or non-obvious usage

### Private Items

| Item | Summary | Details | Examples |
|------|---------|---------|----------|
| Module | Required | Recommended | Optional |
| Struct/Enum | Required | Optional | Optional |
| Function | Required | Optional | Optional |
| Complex Logic | Required | Required | Optional |

### Field and Variant Documentation

- **Struct fields**: Document only if `pub` OR purpose is not obvious from name
- **Enum variants (public)**: All variants must be documented
- **Enum variants (private)**: Document only if purpose is not obvious from name

---

## Crate Maturity Levels

### Mature/Stable Crates

Full documentation required for all public items. Private items should have at least summary documentation.

### Experimental/Unstable Crates

Focus on public API entry points. Private modules need only brief purpose explanations.

---

## Standard Sections

Use sections in this order:

1. Summary (no header, always first)
2. Extended description (no header)
3. `# Arguments` - Required for functions with parameters
4. `# Returns` - Required for non-obvious return values
5. `# Errors` - Required for Result-returning functions
6. `# Panics` - Required if function can panic
7. `# Safety` - Required for unsafe code
8. `# Examples` - Required for complex APIs

### Section Formatting

```rust
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value.
///
/// # Errors
///
/// Returns an error if:
/// - Condition A occurs
/// - Condition B occurs
```

---

## Example Best Practices

1. **Keep examples minimal** - Show the concept, not everything
2. **Use meaningful values** - Use domain-relevant names instead of `foo`, `bar`, `test`
3. **Use `?` for error handling** - Wrap in hidden `main` function
4. **Use assertions** - `assert_eq!`, `assert!` demonstrate expected behavior
5. **Hide boilerplate with `#`** - Hide obvious `use` statements and `main` wrapper

### Example with Result

```rust
/// # Examples
///
/// ```
/// # use crate::{process, Config};
/// # fn main() -> Result<(), Error> {
/// let result = process("input", Config::default())?;
/// assert!(result.is_valid());
/// # Ok(())
/// # }
/// ```
```

### Example without Result

```rust
/// # Examples
///
/// ```
/// # use crate::MyStruct;
/// let instance = MyStruct::new("example");
/// assert_eq!(instance.name(), "example");
/// ```
```

---

## ASCII Art Diagrams

For complex concepts, include ASCII diagrams:

```rust
//! # Data Flow
//!
//! ```text
//! Source Text
//!     ↓ lexer
//! Tokens
//!     ↓ parser
//! AST
//!     ↓ transform
//! Output
//! ```
```

---

## Templates

### Module

```rust
//! Brief one-line description.
//!
//! Longer explanation of what this module provides.
//!
//! # Overview
//!
//! - [`MainType`] - Primary type description
//! - [`helper_fn`] - Helper function description
//!
//! # Example
//!
//! ```
//! # use crate::module::MainType;
//! let instance = MainType::new();
//! assert_eq!(instance.value(), 0);
//! ```
```

### Struct

```rust
/// Brief one-line description.
///
/// Longer description explaining:
/// - What this type represents
/// - When to use it
///
/// # Examples
///
/// ```
/// # use crate::MyStruct;
/// let instance = MyStruct::new(42);
/// assert_eq!(instance.value(), 42);
/// ```
pub struct MyStruct {
    /// Document public fields.
    pub value: i32,
    // Private obvious fields need no docs
    count: usize,
}

impl MyStruct {
    /// Creates a new instance with the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The initial value
    pub fn new(value: i32) -> Self {
        Self { value, count: 0 }
    }

    /// Returns the current value.
    pub fn value(&self) -> i32 {
        self.value
    }
}
```

### Enum (Public)

```rust
/// Brief one-line description.
///
/// Explanation of what states this enum represents.
pub enum Status {
    /// Waiting for input.
    Pending,
    /// Currently processing with progress percentage.
    Running(u8),
    /// Completed successfully.
    Done,
    /// Failed with error message.
    Failed(String),
}
```

### Enum (Private)

```rust
/// Processing state machine.
enum State {
    Idle,
    Running,
    /// Waiting for external resource with timeout in milliseconds.
    Waiting(u64),
    Done,
}
```

### Trait

```rust
/// Brief one-line description.
///
/// Explanation of what this trait abstracts.
///
/// # Implementing
///
/// Implementors must ensure:
/// - Requirement 1
/// - Requirement 2
///
/// # Example
///
/// ```
/// # use crate::MyTrait;
/// struct MyType;
///
/// impl MyTrait for MyType {
///     fn process(&self) -> i32 {
///         42
///     }
/// }
///
/// let instance = MyType;
/// assert_eq!(instance.process(), 42);
/// ```
pub trait MyTrait {
    /// Processes and returns a value.
    ///
    /// # Returns
    ///
    /// The computed value.
    fn process(&self) -> i32;

    /// Optional method with default implementation.
    fn is_ready(&self) -> bool {
        true
    }
}
```

### Function

```rust
/// Brief one-line description.
///
/// Longer description if needed.
///
/// # Arguments
///
/// * `input` - The data to process
/// * `options` - Configuration options
///
/// # Returns
///
/// The processed output.
///
/// # Errors
///
/// Returns an error if:
/// - Input is invalid
/// - Processing fails
///
/// # Examples
///
/// ```
/// # use crate::{process, Options};
/// # fn main() -> Result<(), Error> {
/// let result = process("data", Options::default())?;
/// assert!(result.is_valid());
/// # Ok(())
/// # }
/// ```
pub fn process(input: &str, options: Options) -> Result<Output, Error> {
    // implementation
}
```

### Private Complex Logic

```rust
/// Calculates intersection point of a line with a rectangle boundary.
///
/// Uses parametric line equations to find where the line from `a` to `b`
/// crosses the rectangle centered at `a` with the given `size`.
///
/// Algorithm:
/// 1. Calculate parametric t values for each edge
/// 2. Filter to valid intersections (0 < t < 1)
/// 3. Return the closest intersection point
fn find_intersection(a: Point, b: Point, size: Size) -> Point {
    // implementation
}
```

---

## Consistency Rules

### Rule 1: Every `.rs` file starts with module docs

```rust
//! Brief description of the module.
```

### Rule 2: Every `pub` item has documentation

```rust
/// Brief description.
pub struct MyStruct { ... }

/// Brief description.
pub fn my_function() { ... }
```

### Rule 3: Use intra-doc links

```rust
/// See [`OtherType`] for details.
/// Returns a [`Result`] containing [`Output`].
```

### Rule 4: Use backticks for code

Always wrap code snippets, variable names, and constants in backticks:

```rust
/// The `config` parameter controls behavior.
/// Returns `None` if the key is not found.
/// Set `MAX_RETRIES` to configure retry limit.
```

### Rule 5: Complete sentences end with `.`
All documentation sentences must end with a period:

```rust
/// Returns the current value.
///
/// # Arguments
///
/// * `key` - The lookup key.
```

### Rule 6: Consistent terminology

| Preferred | Avoid |
|-----------|-------|
| "Returns" | "Return value is" |
| "Creates" | "This function creates" |
| "Panics if" | "Will panic when" |
