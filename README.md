# Orrery

A diagram language for creating component and sequence diagrams with a simple text-based DSL.

**Website:** [orreryworks.github.io](https://orreryworks.github.io/)

## Overview

Orrery is a domain-specific language for describing software architecture diagrams. Write diagrams in a simple text format and render them to SVG.

**Supported diagram types:**
- Component diagrams
- Sequence diagrams

## Why "Orrery"?

An [orrery](https://en.wikipedia.org/wiki/Orrery) is a mechanical model of the solar system — a clockwork device that makes the invisible relationships and movements of celestial bodies visible and tangible. In the same spirit, Orrery the language makes the structure and interactions within software systems visible through diagrams.

## Workspace Structure

This project is organized as a Cargo workspace with four crates:

- **`orrery`** - Main library for layout and rendering
- **`orrery-core`** - Core types and definitions for Orrery diagrams
- **`orrery-parser`** - Parser for the Orrery diagram language
- **`orrery-cli`** - Command-line interface built on the library

## Library Usage

Add `orrery` to your `Cargo.toml`.

### Basic Example

```rust
use orrery::DiagramBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        diagram sequence;
        
        client: Actor;
        app: Component;
        
        client -> app: "Get Request";
        client <- app: "Response";
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

- [Docs](https://orreryworks.github.io/docs/) - Getting started, language reference, and CLI manual
- [API Documentation](https://docs.rs/orrery) - Library API docs
- [Examples](https://orreryworks.github.io/docs/examples/component-diagrams.html) - Component, sequence, and cross-cutting diagrams

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/orreryworks/orrery.git
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
