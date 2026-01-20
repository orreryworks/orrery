# Error Message Style Guide

This document defines the conventions for error messages in Filament, following idiomatic Rust style as established by rustc, Clippy, and rust-analyzer.

## Message Format

### Capitalization

**Use lowercase** for the start of error messages, unless the first word is a proper noun or an identifier that is capitalized in source code.

```
✓ "cannot find type `Foo` in this scope"
✓ "component `user` not found"
✓ "invalid attribute value"

✗ "Cannot find type `Foo` in this scope"
✗ "Component `user` not found"
✗ "Invalid attribute value"
```

### Code References

**Use backticks** for identifiers, types, keywords, and code snippets. Do not use single quotes or double quotes.

```
✓ "component `server` not found"
✓ "type `Rectangle` is defined multiple times"
✓ "expected `;`, found `}`"

✗ "component 'server' not found"
✗ "type "Rectangle" is defined multiple times"
✗ "component server not found"
```

### Punctuation

**No trailing period** on error messages. Keep messages concise.

```
✓ "undefined component"
✓ "invalid color value"

✗ "undefined component."
✗ "invalid color value."
```

### Tone

**Be direct and concise**. State what is wrong without unnecessary words.

```
✓ "component `foo` not found"
✓ "duplicate type definition"
✓ "nested diagram not allowed"

✗ "the component named `foo` could not be found in the current scope"
✗ "there is already a type definition with this name"
✗ "it is not allowed to nest diagrams inside other diagrams"
```

## Help Text

Help text provides actionable guidance. It can be slightly more verbose than the main message.

### Format

- Start with lowercase
- No trailing period
- Provide specific, actionable advice

```
✓ "valid values: left, right, top, bottom"
✓ "component must be defined before it can be referenced"
✓ "use a valid CSS color"

✗ "The valid values are: left, right, top, bottom."
✗ "You need to define the component before you can reference it."
```

## Labels

Labels annotate specific source locations. They should be brief.

```
✓ "undefined component"
✓ "duplicate definition"
✓ "first defined here"
✓ "invalid value"

✗ "this component is undefined"
✗ "this is a duplicate definition"
```

## Error Code Organization

Error codes are organized by phase:

| Range | Phase | Example |
|-------|-------|---------|
| E0xx | Lexer | E001: unterminated string literal |
| E1xx | Parser | E100: unexpected token |
| E2xx | Validation | E200: undefined component |
| E3xx | Elaboration | E300: undefined type |

## Examples

### Good Error Message

```
error[E200]: component `server` not found
  --> src/diagram.fil:10:5
   |
10 |     user -> server: "request";
   |             ^^^^^^ undefined component
   |
   = help: component must be defined before it can be referenced
```

### Multiple Labels

```
error[E301]: type `ApiService` is defined multiple times
  --> src/diagram.fil:15:1
   |
15 | type ApiService = Rectangle;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ duplicate definition
   |
  --> src/diagram.fil:8:1
   |
 8 | type ApiService = Circle;
   | ------------------------- first defined here
   |
   = help: remove the duplicate or use a different name
```

## References

- [rustc Error Index](https://doc.rust-lang.org/error-index.html)
- [Rust API Guidelines - Error Messages](https://rust-lang.github.io/api-guidelines/interoperability.html)
- [Clippy Lint Documentation](https://rust-lang.github.io/rust-clippy/master/)