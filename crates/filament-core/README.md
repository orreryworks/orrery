# Filament Core

Foundational types and definitions for the [Filament](https://github.com/foadnh/filament) diagram language.

This crate provides the shared core types used across the Filament ecosystem.

## Modules

- **`identifier`** — Efficient string-interned identifiers ([`Id`](https://docs.rs/filament-core/latest/filament_core/identifier/struct.Id.html))
- **`color`** — Color handling with CSS color name support ([`Color`](https://docs.rs/filament-core/latest/filament_core/color/struct.Color.html))
- **`geometry`** — Basic geometric primitives: `Point`, `Size`, `Bounds`, `Insets`
- **`draw`** — Visual element definitions for rendering (shapes, arrows, text, strokes, layers)
- **`semantic`** — Semantic model types representing parsed diagrams (`Diagram`, `Node`, `Relation`, `Scope`)

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
