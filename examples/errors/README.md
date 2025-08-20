# Filament Error Examples

This directory contains example files demonstrating Filament's error reporting system. For comprehensive information about error handling, see the [**Error Handling Specification**](../../docs/specifications/error_handling.md).

## Error Examples

### Missing Semicolons
- [`missing_semicolon_component.fil`](missing_semicolon_component.fil) - Missing semicolon after component definition
- [`missing_semicolon_type.fil`](missing_semicolon_type.fil) - Missing semicolon after type definition
- [`missing_relation_semicolon.fil`](missing_relation_semicolon.fil) - Missing semicolon after relation
- [`simple_semicolon.fil`](simple_semicolon.fil) - Basic missing semicolon example
- [`component_semicolon.fil`](component_semicolon.fil) - Component-specific semicolon error
- [`type_semicolon.fil`](type_semicolon.fil) - Type definition semicolon error
- [`relation_semicolon.fil`](relation_semicolon.fil) - Relation semicolon error

### Missing Colons
- [`missing_colon_component.fil`](missing_colon_component.fil) - Missing colon in component definition

### Missing Brackets/Braces
- [`missing_bracket.fil`](missing_bracket.fil) - Missing closing bracket in attributes
- [`bracket_only.fil`](bracket_only.fil) - Simple bracket error example

### Invalid Syntax
- [`invalid_diagram_header.fil`](invalid_diagram_header.fil) - Incomplete diagram header
- [`keyword_typo.fil`](keyword_typo.fil) - Common keyword typos
- [`error_example.fil`](error_example.fil) - Complex multi-error example

### Activation Errors (Blocks and Explicit Statements)
- [`activation_in_component_diagram_block.fil`](activation_in_component_diagram_block.fil) - Activation (block form) used in a component diagram (invalid)
- [`activation_in_component_diagram_explicit.fil`](activation_in_component_diagram_explicit.fil) - Activation (explicit form) used in a component diagram (invalid)
- [`deactivate_without_activate.fil`](deactivate_without_activate.fil) - Deactivate without a matching prior activate
- [`unpaired_activate_end_of_scope.fil`](unpaired_activate_end_of_scope.fil) - Unpaired activate at end of scope

### Fragment Errors
- [`fragment_in_component_diagram.fil`](fragment_in_component_diagram.fil) - Fragment used in a component diagram (invalid). File also includes commented syntax-error variants (missing section semicolon, missing fragment operation string, missing closing brace).

### Complex Scenarios
- [`complex_nested.fil`](complex_nested.fil) - Error in complex nested architecture

### Valid Examples
- [`simple_valid.fil`](simple_valid.fil) - Simple valid syntax example
- [`valid_example.fil`](valid_example.fil) - Complex valid example

## Usage

Run any example to see the error messages:

```bash
cargo run examples/errors/missing_semicolon_component.fil
```
