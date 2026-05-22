# Orrery Documentation Guidelines

## Philosophy

This guide extends the [Rust API Guidelines — Documentation](https://rust-lang.github.io/api-guidelines/documentation.html) with Orrery-specific style and a strong bias toward brevity.

A doc comment that only restates the signature is noise. Public items always carry at least a one-line summary; depth scales with complexity, not with ceremony. The shape of a good comment is: one line of summary plus whatever the reader genuinely needs — errors, panics, invariants, a substantive example.

## Core Principles

1. **Signal over volume.** Aim for the shortest comment a future contributor would still thank you for.
2. **Document what the code cannot say.** Intent, invariants, edge cases, tradeoffs, surprising behavior. The signature speaks for itself about names and types.
3. **Every public item is documented; depth scales with complexity.** A trivial accessor may carry only a one-line summary; a non-obvious API earns full prose, an `# Errors` section, and a substantive example.
4. **Document dependencies, not dependents.** Reference upstream types; never mention downstream consumers — those relationships change and the comment will lie.
5. **Examples show *why*, not just *how*.** Aim to include an example on public items where it adds value, per [C-EXAMPLE](https://rust-lang.github.io/api-guidelines/documentation.html#examples-use-error-not-tryunwrap-c-question-mark). A link to an example on a related item is an acceptable substitute.
6. **Consistency in style, not in length.** The patterns are consistent; the lengths of real comments are not.

## Anti-Patterns

### Restating the signature

```rust
/// Returns the name.
///
/// # Returns
///
/// The name as a `String`.
pub fn name(&self) -> String { ... }
```

Either omit the comment or surface something the signature doesn't:

```rust
/// Returns the display name, falling back to the login if unset.
pub fn name(&self) -> String { ... }
```

### Narrating the obvious

```rust
/// This struct represents a user.
///
/// It has a name and an age.
pub struct User {
    /// The name of the user.
    pub name: String,
    /// The age of the user.
    pub age: u32,
}
```

One line for the type; field comments only when the name doesn't carry the meaning:

```rust
/// An authenticated end-user account.
pub struct User {
    pub name: String,
    pub age: u32,
}
```

### Ceremonial section headers with empty content

```rust
/// Adds two numbers.
///
/// # Arguments
///
/// * `a` - The first number.
/// * `b` - The second number.
///
/// # Returns
///
/// The sum.
pub fn add(a: i32, b: i32) -> i32 { a + b }
```

Use `# Arguments` / `# Returns` only when they carry meaning beyond the signature:

```rust
/// Returns `a + b`, wrapping on overflow.
pub fn add(a: i32, b: i32) -> i32 { ... }
```

### Filler examples

```rust
/// # Examples
///
/// ```
/// let x = MyStruct::new();
/// ```
```

An example that only invokes the constructor adds no information.

### "This function..." preambles

Start summaries with the verb or noun directly.

| Avoid                                          | Prefer                          |
| ---------------------------------------------- | ------------------------------- |
| `/// This function parses the input.`          | `/// Parses the input.`         |
| `/// This struct represents a configuration.`  | `/// Runtime configuration.`    |
| `/// The return value is the number of bytes.` | `/// Returns the byte count.`   |

### Module docs that re-list every item

Rustdoc already produces a per-kind index of every `pub` item. A module doc that flat-enumerates the same items with their first-line summaries duplicates that index and goes stale. Module docs are for purpose, conceptual groupings, and cross-cutting invariants — curated overviews like [`std::collections`](https://doc.rust-lang.org/std/collections/) are welcome; verbatim enumerations are not.

## When a Doc Comment Is Not Needed

Public items always carry at least a one-line doc. Doc comments are not needed for:

- Derived trait impls (`Debug`, `Clone`, etc.).
- Private fields with self-explanatory names.
- Private enum variants with self-explanatory names.
- Private helpers with short bodies and descriptive names.

For a public trivial accessor or a public `new` whose fields are documented on the struct, a one-line summary suffices and the example may link to the parent type's.

## Documentation Syntax

| Syntax | Purpose                  | Location       |
| ------ | ------------------------ | -------------- |
| `//!`  | Module/crate-level docs  | Top of file    |
| `///`  | Item-level docs          | Above items    |
| `//`   | Implementation comments  | Within code    |

## What to Document, by Item Type

### Public items

| Item                | Summary  | Details when non-obvious                          | Example                                                      |
| ------------------- | -------- | ------------------------------------------------- | ------------------------------------------------------------ |
| Module / crate      | Required | Recommended                                       | Recommended for crate root and entry-point modules           |
| Struct / Enum       | Required | When purpose isn't obvious from name              | Recommended (may link to an example on a constructor/method) |
| Trait               | Required | Required (what it abstracts, invariants)          | Recommended (may link to an example on a method)             |
| Function / Method   | Required | When behavior or errors aren't obvious            | Recommended where it adds value; may link to a related item  |
| Public field        | Required | Only if the name doesn't fully explain it         | —                                                            |
| Public enum variant | Required | Only when the variant carries non-obvious meaning | —                                                            |

### Private items

Documented when the *purpose* or *algorithm* isn't obvious from the code. A one-line comment is usually enough; short, well-named helpers need none.

## Standard Sections

In order, included only when they have content:

1. Summary line (no header, always first).
2. Extended description (no header).
3. `# Arguments` — when an argument needs explanation beyond its name and type.
4. `# Returns` — when the return needs explanation beyond its type.
5. `# Errors` — required for `Result`-returning functions; list the conditions that produce each error variant.
6. `# Panics` — required when the function can panic.
7. `# Safety` — required for `unsafe` functions.
8. `# Examples` — per C-EXAMPLE.

### Section formatting

```rust
/// # Arguments
///
/// * `key` - The lookup key
/// * `mode` - Open mode; `Write` truncates, `Append` does not.
///
/// # Errors
///
/// - [`Error::NotFound`] if the key is absent.
/// - [`Error::Io`] if the underlying read fails.
```

## Examples: Best Practices

1. **Show *why*, not just *how*.** Demonstrate a reason to reach for the item.
2. **Keep examples minimal.** One concept per example.
3. **Use domain-relevant names.** Prefer `tokens`, `config`, `node` over `foo`, `bar`, `baz`.
4. **Hide boilerplate with `#`.** Hide `use` statements and `main` wrappers.
5. **Use `?`, not `unwrap`** for fallible code, per [C-QUESTION-MARK](https://rust-lang.github.io/api-guidelines/documentation.html#examples-use-error-not-tryunwrap-c-question-mark).
6. **Use assertions** to demonstrate expected behavior.
7. **Link instead of duplicate.** Write the example once; link the rest to it.

### With `Result`

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

### Without `Result`

```rust
/// # Examples
///
/// ```
/// # use crate::Lexer;
/// let tokens = Lexer::new("1 + 2").tokenize();
/// assert_eq!(tokens.len(), 3);
/// ```
```

## ASCII Diagrams

Use diagrams for pipelines, state machines, and data flows when they convey structure better than prose.

```rust
//! ```text
//! Source ──► Lexer ──► Parser ──► AST ──► Codegen
//! ```
```

## Templates

Upper bounds, not minima. Drop sections that don't carry information.

### Module

```rust
//! What this module is for, in one or two sentences.
//!
//! Optional: a paragraph on the key concept or invariant the module enforces.
```

Entry-point module:

```rust
//! Tokenization for the Orrery query language.
//!
//! The lexer is single-pass and allocation-free for ASCII input.
//!
//! ```text
//! &str ──► Lexer ──► Iterator<Token>
//! ```
```

### Struct

```rust
/// One-line description of what this represents.
pub struct Config {
    pub max_retries: u32,
    /// Timeout per attempt; `None` disables the per-attempt deadline.
    pub timeout: Option<Duration>,
}
```

Place the `# Examples` block on the type or on its primary constructor (with a link from the type).

### Enum (public)

```rust
/// Result of a single parse attempt.
pub enum ParseOutcome {
    Success(Ast),
    /// Parsing failed but recovery produced a partial AST usable for diagnostics.
    Recovered(Ast, Vec<Diagnostic>),
    Failed(Vec<Diagnostic>),
}
```

Document a variant only when its meaning isn't obvious from name and payload.

### Trait

```rust
/// One-line description of the abstraction.
///
/// # Implementing
///
/// Include this section when implementors must uphold non-obvious invariants
/// or ordering guarantees.
pub trait Sink {
    /// Writes a chunk. Returns the number of bytes consumed; a short write
    /// is permitted and the caller must retry the remainder.
    fn write_chunk(&mut self, buf: &[u8]) -> Result<usize>;
}
```

### Function

```rust
/// Parses `input` into an [`Ast`].
///
/// # Errors
///
/// - [`ParseError::UnexpectedToken`] when a token doesn't match the grammar.
/// - [`ParseError::Eof`] when input ends mid-expression.
///
/// # Examples
///
/// ```
/// # use crate::parse;
/// let ast = parse("1 + 2")?;
/// assert_eq!(ast.root_kind(), NodeKind::BinaryOp);
/// # Ok::<_, ParseError>(())
/// ```
pub fn parse(input: &str) -> Result<Ast, ParseError> { ... }
```

No `# Arguments` / `# Returns`: the signature conveys both. For a helper covered by another item's example, link instead of duplicating.

### Private complex logic

Explain *why* the code looks the way it does — reasoning, constraints, alternatives rejected — not what each line does:

```rust
/// Finds where the segment from `a` to `b` exits the rectangle centered at
/// `a`. Used for edge routing — we need the boundary intersection, not the
/// nearest grid point, so a parametric solve beats sampling.
fn find_exit(a: Point, b: Point, size: Size) -> Point { ... }
```

## Consistency Rules

### Every `.rs` file starts with a module doc

One line is fine:

```rust
//! Token definitions for the lexer.
```

### Every `pub` item has at least a one-line doc

A short summary is enough.

### Use intra-doc links

```rust
/// See [`Lexer`] for the token producer.
/// Returns a [`Result`] containing an [`Ast`].
```

### Use backticks for code

```rust
/// The `config` parameter controls retries.
/// Returns `None` when the key is absent.
```

### Punctuation

Prose sentences end with `.`. Bullet items in `# Arguments`, `# Returns`, and `# Errors` end with `.` only when they're full sentences or clauses; short noun-phrase descriptions don't need one.

```rust
/// # Arguments
///
/// * `path` - Filesystem path
/// * `opts` - Open options; `read` and `write` may both be set.
///
/// # Errors
///
/// - [`io::Error`] if the path can't be opened.
```

### Preferred terminology

| Preferred              | Avoid                          |
| ---------------------- | ------------------------------ |
| `Returns ...`          | `The return value is ...`      |
| `Creates ...`          | `This function creates ...`    |
| `Panics if ...`        | `Will panic when ...`          |
| `Parses ...`           | `This method parses ...`       |
