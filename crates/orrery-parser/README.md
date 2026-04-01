# Orrery Parser

Parser for the [Orrery](https://github.com/orreryworks/orrery) diagram language. This crate provides the complete parsing pipeline from source text to semantic diagram representation.

## Overview

The parser processes Orrery source code through a multi-stage pipeline:

1. **Resolve** — Recursively load the root file and all its imports via a [`SourceProvider`](https://docs.rs/orrery-parser/latest/orrery_parser/source_provider/trait.SourceProvider.html), building a virtual address space and populating the import tree. For each file:
   - **Tokenize** — Convert source text to tokens
   - **Parse** — Build an AST from tokens
2. **Desugar** — Normalize syntax sugar and flatten imported types
3. **Validate** — Check semantic validity
4. **Elaborate** — Transform to the semantic model ([`orrery_core::semantic::Diagram`](https://docs.rs/orrery-core/latest/orrery_core/semantic/struct.Diagram.html))

## Quick Example

```rust
use std::path::Path;
use orrery_parser::{parse, ElaborateConfig, InMemorySourceProvider, error::ParseError};

fn main() -> Result<(), ParseError> {
    let source = r#"
        diagram component;
        user: Rectangle;
        server: Rectangle;
        user -> server: "Request";
    "#;

    let mut provider = InMemorySourceProvider::new();
    provider.add_file("main.orr", source);

    let diagram = parse(Path::new("main.orr"), provider, ElaborateConfig::default())?;
    println!("Diagram kind: {:?}", diagram.kind());
    Ok(())
}
```

## Documentation

- [API Documentation](https://docs.rs/orrery-parser)

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
