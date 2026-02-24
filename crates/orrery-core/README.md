# Orrery Core

Foundational types and definitions for the [Orrery](https://github.com/orreryworks/orrery) diagram language.

This crate provides the shared core types used across the Orrery ecosystem.

## Modules

- **`identifier`** — Efficient string-interned identifiers ([`Id`](https://docs.rs/orrery-core/latest/orrery_core/identifier/struct.Id.html))
- **`color`** — Color handling with CSS color name support ([`Color`](https://docs.rs/orrery-core/latest/orrery_core/color/struct.Color.html))
- **`geometry`** — Basic geometric primitives: `Point`, `Size`, `Bounds`, `Insets`
- **`draw`** — Visual element definitions for rendering (shapes, arrows, text, strokes, layers)
- **`semantic`** — Semantic model types representing parsed diagrams (`Diagram`, `Node`, `Relation`, `Scope`)

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
