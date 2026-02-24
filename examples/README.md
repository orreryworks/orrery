# Orrery Examples

## Running Examples

```bash
cargo run --release -- examples/<example>.orr
```

The generated SVG is saved as `out.svg` in the current directory.

## Examples

### Component Diagrams

| File | Features |
|------|----------|
| [`component_basic.orr`](component_basic.orr) | Component definitions, display names (`as "..."`), relation types (`->`, `<-`, `<->`, `-`), labels |
| [`component_shapes.orr`](component_shapes.orr) | All built-in shapes: Rectangle, Oval, Component, Actor, Entity, Control, Interface, Boundary; content-free vs content-supporting |
| [`component_nesting.orr`](component_nesting.orr) | Nested components, multi-level nesting, cross-level relations (`parent::child`) |
| [`component_layout_engines.orr`](component_layout_engines.orr) | Basic vs Sugiyama layout engines side-by-side |

### Sequence Diagrams

| File | Features |
|------|----------|
| [`sequence_basic.orr`](sequence_basic.orr) | Participants, styled messages, relation types, self-messages, custom arrow types |
| [`sequence_activation.orr`](sequence_activation.orr) | Block-form activation, explicit activate/deactivate, deep nesting, stacked activation (same participant), mixed usage, custom activation types |
| [`sequence_fragments.orr`](sequence_fragments.orr) | Base `fragment`/`section` syntax, sugar keywords (`alt`, `opt`, `loop`, `par`, `break`, `critical`), nested fragments, fragment styling |
| [`sequence_notes.orr`](sequence_notes.orr) | Attached notes, spanning notes, margin notes, alignment (`over`, `left`, `right`), styling, custom note types, notes inside activations and fragments |

### Cross-Cutting

| File | Features |
|------|----------|
| [`type_system.orr`](type_system.orr) | `type` declarations, composition and extension, attribute group types (`Stroke`, `Text`), declarations (`:`) vs invocations (`@`), named vs anonymous TypeSpec, sugar syntax |
| [`styling.orr`](styling.orr) | Stroke attributes (color, width, style, dash patterns, cap, join), text attributes (font_size, font_family, color, background_color, padding), color formats (named, hex, rgb, rgba) |
| [`embedded_diagrams.orr`](embedded_diagrams.orr) | Embedding sequence and component diagrams inside components, layout engines on embedded diagrams, styled embedded content |

### Error Examples

See [`errors/`](errors/) for examples demonstrating Orrery's error reporting.