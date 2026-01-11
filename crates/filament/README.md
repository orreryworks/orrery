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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        diagram component;
        
        frontend: Rectangle [fill_color="lightblue"];
        backend: Rectangle [fill_color="lightgreen"];
        database: Rectangle;
        
        frontend -> backend: "API";
        backend -> database: "SQL";
    "#;

    // Create a builder with default configuration
    let builder = DiagramBuilder::default();

    // Parse the source code into a semantic diagram
    let diagram = builder.parse(source)?;

    // Render the semantic diagram to SVG
    let svg = builder.render_svg(&diagram)?;

    std::fs::write("output.svg", &svg)?;
    Ok(())
}
```

## Custom Configuration

```rust
use filament::{DiagramBuilder, config::AppConfig};

let config = AppConfig::default();
let builder = DiagramBuilder::new(config);
```

## Examples

See the [examples directory](../../examples/) for more diagram samples.

## Documentation

- [Language Specification](../../docs/specifications/specification.md)
- [API Documentation](https://docs.rs/filament)

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
