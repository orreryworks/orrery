# Orrery Error Examples

Example files that trigger specific compiler errors. Each file targets one error.

For error handling details, see the [Error Handling Specification](../../docs/specifications/error_handling.md).

## Usage

```bash
cargo run examples/errors/<file>.orr
```

## Lexer Errors

| File | Code | Error |
|------|------|-------|
| [`lexer_unterminated_string.orr`](lexer_unterminated_string.orr) | E001 | String literal opened with `"` but never closed |
| [`lexer_unexpected_character.orr`](lexer_unexpected_character.orr) | E002 | Character not valid in the Orrery language (`$`) |
| [`lexer_invalid_escape.orr`](lexer_invalid_escape.orr) | E003 | Unrecognized escape sequence (`\x`) in a string literal |
| [`lexer_invalid_unicode_escape.orr`](lexer_invalid_unicode_escape.orr) | E004 | Malformed unicode escape — missing braces (`\u1F602` instead of `\u{1F602}`) |
| [`lexer_invalid_unicode_codepoint.orr`](lexer_invalid_unicode_codepoint.orr) | E005 | Unicode codepoint out of valid range (`\u{110000}` exceeds `0x10FFFF`) |
| [`lexer_empty_unicode_escape.orr`](lexer_empty_unicode_escape.orr) | E006 | Empty unicode escape — no hex digits inside braces (`\u{}`) |

## Parse Errors

| File | Code | Error |
|------|------|-------|
| [`parse_missing_semicolon_component.orr`](parse_missing_semicolon_component.orr) | E100 | Missing `;` after component definition |
| [`parse_missing_semicolon_type.orr`](parse_missing_semicolon_type.orr) | E100 | Missing `;` after type definition |
| [`parse_missing_semicolon_relation.orr`](parse_missing_semicolon_relation.orr) | E100 | Missing `;` after relation |
| [`parse_missing_semicolon_nested.orr`](parse_missing_semicolon_nested.orr) | E100 | Missing `;` inside a nested `{}` block |
| [`parse_missing_colon.orr`](parse_missing_colon.orr) | E100 | Missing `:` in component definition |
| [`parse_missing_bracket.orr`](parse_missing_bracket.orr) | E100 | Missing closing `]` in attribute block |
| [`parse_invalid_diagram_header.orr`](parse_invalid_diagram_header.orr) | E100 | `diagram;` without type (component/sequence) |
| [`parse_keyword_typo.orr`](parse_keyword_typo.orr) | E100 | Unrecognized keyword (`diagramm componnet`) |

## Validation Errors

| File | Code | Error |
|------|------|-------|
| [`validate_undefined_component.orr`](validate_undefined_component.orr) | E200 | Relation references a component that was never declared |
| [`validate_unpaired_activate.orr`](validate_unpaired_activate.orr) | E201 | `activate` with no matching `deactivate` before end of scope |
| [`validate_deactivate_without_activate.orr`](validate_deactivate_without_activate.orr) | E202 | `deactivate` without a matching prior `activate` |
| [`validate_invalid_align.orr`](validate_invalid_align.orr) | E203 | Invalid `align` value for the diagram type (e.g., `"top"` in sequence diagram) |

## Elaboration Errors

| File | Code | Error |
|------|------|-------|
| [`elab_undefined_type.orr`](elab_undefined_type.orr) | E300 | Reference to undefined base type (`rectangle` instead of `Rectangle`) |
| [`elab_duplicate_type.orr`](elab_duplicate_type.orr) | E301 | Type defined more than once with the same name |
| [`elab_invalid_attribute_value.orr`](elab_invalid_attribute_value.orr) | E302 | Attribute value is not valid for the expected type (e.g., `rounded="not-a-number"`) |
| [`elab_unknown_attribute.orr`](elab_unknown_attribute.orr) | E303 | Attribute name not recognized for the shape type (e.g., `bogus="value"`) |
| [`elab_activation_in_component.orr`](elab_activation_in_component.orr) | E304 | Activation block used in a component diagram (sequence-only feature) |
| [`elab_fragment_in_component.orr`](elab_fragment_in_component.orr) | E304 | Fragment block used in a component diagram (sequence-only feature) |
| [`elab_type_mismatch.orr`](elab_type_mismatch.orr) | E307 | Type used in wrong context (e.g., Arrow type as a component shape) |
| [`elab_content_in_content_free_shape.orr`](elab_content_in_content_free_shape.orr) | E308 | Nested content inside a content-free shape (Actor, Entity, Control, Interface, Boundary) |

## Unreachable Error Codes

The following error codes are defined in the system but cannot be triggered via `.orr` input.
They are defensive guards that protect against invalid internal states.

| Code | Description | Reason |
|------|-------------|--------|
| E101 | Incomplete input | Maps to `ErrMode::Incomplete` (streaming parsers only); all truncated inputs produce E100 instead |
| E305 | Nested diagram not allowed | Parser never places a `Diagram` element where the elaborator would detect nesting |
| E306 | Invalid diagram structure | Parser always produces a valid `Diagram` at the top level |
| E309 | Diagram cannot share scope | Parser does not allow standalone `embed diagram` as a top-level element |
