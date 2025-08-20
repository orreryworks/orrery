# Filament Examples

This directory contains example Filament files demonstrating various features of the language.

## Running Examples

To run any example:

```bash
cargo run --release -- examples/<example_name>.fil
```

The generated SVG will be saved as `out.svg` in the current directory.

## Available Examples

### Component Diagrams
- `boundary_example.fil` - Boundary elements and containment
- `component_labels.fil` - Component labeling and styling
- `cross_level_relations.fil` - Relations across different component levels
- `embedded_diagram.fil` - Embedding diagrams within components
- `multi_level_nested.fil` - Multi-level component nesting
- `nested_component.fil` - Basic component nesting
- `shape_comparison.fil` - Different component shapes
- `shape_types_showcase.fil` - Comprehensive shape showcase
- `text_color_showcase.fil` - Text styling and colors
- `text_size_demo.fil` - Text sizing options
- `uml_basic_example.fil` - UML-style component diagram
- `simple_relation_types.fil` - Different relation types and styles

### Sequence Diagrams (Activation: Blocks and Explicit Statements)
- `activate_blocks.fil` - Complex activation using block syntax (sugar for explicit statements)
- `activate_blocks_simple.fil` - Simple activation using block syntax (sugar for explicit statements)
- `activate_explicit.fil` - Activation using explicit statements (activate/deactivate)
- `activate_mixed.fil` - Mixed block and explicit activation usage

### Fragment Blocks
- `fragment_minimal.fil` - Minimal fragment example
- `fragment_authentication_flow.fil` - Authentication flow with multiple sections and nested fragments

### Layout and Styling
- `arrow_styles.fil` - Different arrow and relation styles
- `sugiyama_layout.fil` - Sugiyama layout algorithm demonstration
- `float_test.fil` - Floating point positioning

### Error Examples
- `elaboration_errors.fil` - Common elaboration errors
- `error_example.fil` - General error scenarios
- `uml_error_example.fil` - UML-specific errors
- `errors/` - Directory with additional error examples
