# Filament Library

A Rust library for parsing, layouting, and rendering diagram descriptions written in the Filament DSL.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
filament = "0.1.0"
```

## Quick Start

```rust
use filament::DiagramBuilder;

let source = r#"
    diagram component;
    
    frontend: Rectangle [fill_color="lightblue"];
    backend: Rectangle [fill_color="lightgreen"];
    database: Cylinder;
    
    frontend -> backend [label="API"];
    backend -> database [label="SQL"];
"#;

let svg = DiagramBuilder::new(source)
    .render_svg()
    .expect("Failed to render diagram");

std::fs::write("output.svg", svg)?;
```

## Examples

See the [examples directory](../../examples/) for more diagram samples.

## Documentation

- [Language Specification](../../docs/specifications/specification.md)
- [API Documentation](https://docs.rs/filament)

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
