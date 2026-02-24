# Examples Style Guide

This document defines the conventions for the `examples/` directory. Examples serve as a **feature reference catalog** — each file demonstrates a specific feature working correctly. They are not tutorials.

## Directory Structure

All examples live in a flat directory. Error examples live in `errors/`.

```
examples/
├── README.md
├── component_*.orr
├── sequence_*.orr
├── <cross-cutting>.orr
└── errors/
    ├── README.md
    ├── lexer_*.orr
    ├── parse_*.orr
    ├── elab_*.orr
    └── validate_*.orr
```

## Naming Conventions

### Valid Examples

Use `<kind>_<feature>.orr` naming, where `<kind>` is the diagram kind (`component`, `sequence`, …). The prefix groups files by the diagram kind they target. Files that apply across multiple kinds (type system, styling, embedded diagrams) have no prefix.

```
✓ component_basic.orr
✓ component_shapes.orr
✓ sequence_activation.orr
✓ sequence_notes.orr
✓ type_system.orr
✓ styling.orr

✗ test_fragment.orr
✗ shape_types_showcase.orr
✗ uml_basic_example.orr
✗ float_test.orr
```

Do not use suffixes like `_example`, `_showcase`, `_demo`, or `_test`.

### Error Examples

Use `<phase>_<error>.orr` naming. The phase prefix matches the compiler phase that produces the error.

| Prefix | Phase | Error Code Range |
|--------|-------|------------------|
| `lexer_` | Lexer | E0xx |
| `parse_` | Parser | E1xx |
| `validate_` | Validation | E2xx |
| `elab_` | Elaboration | E3xx |

```
✓ parse_missing_semicolon_component.orr
✓ elab_undefined_type.orr
✓ validate_unpaired_activate.orr
✓ lexer_unterminated_string.orr

✗ error_example.orr
✗ missing_bracket.orr
✗ elaboration_errors.orr
✗ complex_nested.orr
```

## File Template

### Valid Examples

Every `.orr` file follows this structure:

```
// [Feature Name]
//
// Demonstrates:
//   - Capability A
//   - Capability B

diagram <kind>;

// --- [Section] ---

...

// --- [Section] ---

...
```

Use `// --- [Section] ---` dividers to separate logical sections. The number and names of sections vary by file — choose whatever makes sense for the feature being demonstrated.

### Error Examples

Every error `.orr` file follows this structure:

```
// Error [E<code>]: <brief description>
// Phase: <Lexer | Parse | Elaboration | Validation>
//
// The <specific thing> is <specific problem>.
// Expected: <what error the compiler should report>

diagram <kind>;

// ... minimal context ...

// ❌ <inline comment explaining what is wrong>
<offending line>
```

## Content Rules

### One Feature Per File

Each file is a reference for exactly one feature area. Cover simple through advanced usage within that file, separated by `// ---` section dividers.

```
✓ sequence_activation.orr — block form, explicit form, nested, mixed, stacked
✓ component_shapes.orr — all built-in shapes in one file

✗ uml_basic_example.orr — shapes + relations + nesting + styling all in one
```

### One Error Per File

Each error example should target one specific error scenario. A file may contain multiple errors only when demonstrating multi-error reporting (e.g., the compiler recovering and reporting several errors in one pass). Avoid kitchen-sink files that mix unrelated errors from different phases.

```
✓ elab_undefined_type.orr — one type reference error
✓ parse_missing_colon.orr — one parse error
✓ lexer_invalid_escape.orr — primary error cascades a secondary E001

✗ elaboration_errors.orr — six unrelated errors from different phases
```

### Realistic Names

Use domain-relevant identifiers. Avoid single letters and placeholder names.

```
✓ api_gateway, auth_service, users_db
✓ client, server, cache

✗ a, b, c
✗ foo, bar, baz
✗ test1, test2
```

### Concise Comments

State **what**, not why or how. No tutorials, no explanations of language mechanics.

```
✓ // Display name via `as` syntax
✓ // ❌ `rectangle` is not a valid type (should be `Rectangle`)

✗ // The `as` keyword lets you specify an alternative display name
    that will be shown in the rendered diagram instead of the identifier
✗ // This demonstrates how the type system works by showing...
```

### Show Range

Each file should cover simple through advanced usage of its feature area. Arrange sections from basic to complex, separated by `// ---` comment dividers.

## README Maintenance

### `examples/README.md`

Group examples by domain (Component Diagrams, Sequence Diagrams, Cross-Cutting). Use a table with two columns:

| Column | Content |
|--------|---------|
| File | Linked filename |
| Features | Comma-separated list of demonstrated features |

### `examples/errors/README.md`

Group error examples by phase (Lexer, Parse, Validation, Elaboration). Use a table with three columns:

| Column | Content |
|--------|---------|
| File | Linked filename |
| Code | Error code (e.g., E100) |
| Error | One-line description of the error |

Include an **Unreachable Error Codes** section for error codes that are defined in the system but cannot be triggered via `.orr` input. Document why each is unreachable.

## References

- [Error Handling Specification](../specifications/error_handling.md)
- [Error Message Style Guide](error_messages.md)
