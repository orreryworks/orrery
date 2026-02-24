# Orrery

A diagram language for creating component and sequence diagrams with a simple text-based DSL.

## Overview

Orrery is a domain-specific language for describing software architecture diagrams. Write diagrams in a simple text format and render them to SVG.

**Supported diagram types:**
- Component diagrams
- Sequence diagrams

## Workspace Structure

This project is organized as a Cargo workspace with two crates:

- **`orrery`** - Core library for parsing, layout, and rendering
- **`orrery-cli`** - Command-line tool built on the library

## Library Usage

Add `orrery` to your `Cargo.toml`.

### Basic Example

```rust
use orrery::DiagramBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        diagram component;
        
        app: Rectangle [fill_color="blue"];
        db: Cylinder [fill_color="green"];
        cache: Cylinder;
        
        app -> db [label="query"];
        app -> cache [label="read/write"];
    "#;
    
    let svg = DiagramBuilder::new(source)
        .render_svg()?;
    
    std::fs::write("diagram.svg", svg)?;
    Ok(())
}
```

## CLI Usage

### Installation

```bash
cargo install orrery-cli
```

### Basic Usage

```bash
# Render a diagram
orrery input.orr -o output.svg

# With custom config
orrery input.orr -o output.svg --config custom.toml

# With debug logging
orrery input.orr -o output.svg --log-level debug
```

## Documentation

- [Language Specification](docs/specifications/specification.md) - Complete language reference
- [API Documentation](https://docs.rs/orrery) - Library API docs
- [Examples](examples/) - Sample diagrams

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/foadnh/orrery.git
cd orrery

# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Build CLI
cargo build --release
```

### Running Examples

```bash
# Process an example
cargo run -- examples/component_shapes.orr -o output.svg
```

## Contributing

Contributions are welcome! Please see the examples and specification for language details.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
