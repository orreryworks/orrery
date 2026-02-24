# Orrery Parser

Parser for the [Orrery](https://github.com/orreryworks/orrery) diagram language. This crate provides the complete parsing pipeline from source text to semantic diagram representation.

## Overview

The parser processes Orrery source code through a multi-stage pipeline:

1. **Tokenize** — Convert source text to tokens
2. **Parse** — Build AST from tokens
3. **Desugar** — Normalize syntax sugar 
4. **Validate** — Check semantic validity
5. **Elaborate** — Transform to the semantic model ([`orrery_core::semantic::Diagram`](https://docs.rs/orrery-core/latest/orrery_core/semantic/struct.Diagram.html))

## Quick Example

```rust
use orrery_parser::{parse, ElaborateConfig, error::ParseError};

fn main() -> Result<(), ParseError> {
    let source = r#"
        diagram component;
        user: Rectangle;
        server: Rectangle;
        user -> server: "Request";
    "#;

    let diagram = parse(source, ElaborateConfig::default())?;
    println!("Diagram kind: {:?}", diagram.kind());
    Ok(())
}
```

## Documentation

- [API Documentation](https://docs.rs/orrery-parser)

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
