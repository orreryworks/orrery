# Orrery Error Examples

This directory contains example files demonstrating Orrery's error reporting system. For detailed information about error handling, see the [**Error Handling Specification**](../../docs/specifications/error_handling.md).

## Error Examples

### Missing Semicolons
- [`missing_semicolon_component.orr`](missing_semicolon_component.orr) - Missing semicolon after component definition
- [`missing_semicolon_type.orr`](missing_semicolon_type.orr) - Missing semicolon after type definition
- [`missing_relation_semicolon.orr`](missing_relation_semicolon.orr) - Missing semicolon after relation
- [`simple_semicolon.orr`](simple_semicolon.orr) - Basic missing semicolon example
- [`component_semicolon.orr`](component_semicolon.orr) - Component-specific semicolon error
- [`type_semicolon.orr`](type_semicolon.orr) - Type definition semicolon error
- [`relation_semicolon.orr`](relation_semicolon.orr) - Relation semicolon error

### Missing Colons
- [`missing_colon_component.orr`](missing_colon_component.orr) - Missing colon in component definition

### Missing Brackets/Braces
- [`missing_bracket.orr`](missing_bracket.orr) - Missing closing bracket in attributes
- [`bracket_only.orr`](bracket_only.orr) - Simple bracket error example

### Invalid Syntax
- [`invalid_diagram_header.orr`](invalid_diagram_header.orr) - Incomplete diagram header
- [`keyword_typo.orr`](keyword_typo.orr) - Common keyword typos
- [`error_example.orr`](error_example.orr) - Complex multi-error example

### Elaboration Errors
- [`elaboration_errors.orr`](elaboration_errors.orr) - Common elaboration errors (type mismatches, undefined types, etc.)

### Activation Errors (Blocks and Explicit Statements)
- [`activation_in_component_diagram_block.orr`](activation_in_component_diagram_block.orr) - Activation (block form) used in a component diagram (invalid)
- [`activation_in_component_diagram_explicit.orr`](activation_in_component_diagram_explicit.orr) - Activation (explicit form) used in a component diagram (invalid)
- [`deactivate_without_activate.orr`](deactivate_without_activate.orr) - Deactivate without a matching prior activate
- [`unpaired_activate_end_of_scope.orr`](unpaired_activate_end_of_scope.orr) - Unpaired activate at end of scope

### Fragment Errors
- [`fragment_in_component_diagram.orr`](fragment_in_component_diagram.orr) - Fragment used in a component diagram (invalid). File also includes commented syntax-error variants (missing section semicolon, missing fragment operation string, missing closing brace).

### Complex Scenarios
- [`complex_nested.orr`](complex_nested.orr) - Error in complex nested architecture

### Valid Examples
- [`simple_valid.orr`](simple_valid.orr) - Simple valid syntax example
- [`valid_example.orr`](valid_example.orr) - Complex valid example

## Usage

Run any example to see the error messages:

```bash
cargo run examples/errors/missing_semicolon_component.orr
```
