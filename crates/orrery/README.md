# Orrery Library

A Rust library for parsing, layouting, and rendering diagram descriptions written in the Orrery DSL.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
orrery = "0.1.0"
```

## Quick Start

```rust
use std::path::Path;

use orrery::{DiagramBuilder, InMemorySourceProvider, config::AppConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        diagram component;
        
        frontend: Rectangle [fill_color="lightblue"];
        backend: Rectangle [fill_color="lightgreen"];
        database: Rectangle;
        
        frontend -> backend: "API";
        backend -> database: "SQL";
    "#;

    // Set up a source provider with the diagram source
    let mut provider = InMemorySourceProvider::new();
    provider.add_file("diagram.orr", source);

    // Create a builder with default configuration and provider
    let builder = DiagramBuilder::new(AppConfig::default(), &provider);

    // Parse the source code into a semantic diagram
    let diagram = builder.parse(Path::new("diagram.orr"))?;

    // Render the semantic diagram to SVG
    let svg = builder.render_svg(&diagram)?;

    std::fs::write("output.svg", &svg)?;
    Ok(())
}
```

## Custom Configuration

```rust
use orrery::{DiagramBuilder, InMemorySourceProvider, config::AppConfig};

let provider = InMemorySourceProvider::new();
let config = AppConfig::default();
let builder = DiagramBuilder::new(config, &provider);
```

## Examples

See the [examples directory](../../examples/) for more diagram samples.

## Documentation

- [Language Specification](../../docs/specifications/specification.md)
- [API Documentation](https://docs.rs/orrery)

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
