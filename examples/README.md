# Orrery Examples

This directory contains example Orrery files demonstrating various features of the language.

## Running Examples

To run any example:

```bash
cargo run --release -- examples/<example_name>.orr
```

The generated SVG will be saved as `out.svg` in the current directory.

## Available Examples

### Component Diagrams
- `boundary_example.orr` - Boundary elements and containment
- `component_labels.orr` - Component labeling and styling
- `cross_level_relations.orr` - Relations across different component levels
- `embedded_diagram.orr` - Embedding diagrams within components
- `multi_level_nested.orr` - Multi-level component nesting
- `nested_component.orr` - Basic component nesting
- `shape_comparison.orr` - Different component shapes
- `shape_types_showcase.orr` - Comprehensive shape showcase
- `text_color_showcase.orr` - Text styling and colors
- `text_size_demo.orr` - Text sizing options
- `uml_basic_example.orr` - UML-style component diagram
- `simple_relation_types.orr` - Different relation types and styles

### Sequence Diagrams (Activation: Blocks and Explicit Statements)
- `activate_blocks.orr` - Complex activation using block syntax (sugar for explicit statements)
- `activate_blocks_simple.orr` - Simple activation using block syntax (sugar for explicit statements)
- `activate_explicit.orr` - Activation using explicit statements (activate/deactivate)
- `activate_mixed.orr` - Mixed block and explicit activation usage

### Fragment Blocks
- `fragment_minimal.orr` - Minimal fragment example
- `fragment_authentication_flow.orr` - Authentication flow with multiple sections and nested fragments

### Layout and Styling
- `arrow_styles.orr` - Different arrow and relation styles
- `sugiyama_layout.orr` - Sugiyama layout algorithm demonstration
- `float_test.orr` - Floating point positioning

### Error Examples
- `uml_error_example.orr` - UML-specific errors
- `errors/` - Directory with additional error examples
