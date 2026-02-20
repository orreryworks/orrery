# Changelog

All notable changes to the Orrery project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Orrery DSL** — Text-based domain-specific language for describing diagrams.
- **Component diagrams** — Support for component diagram type with nodes, relations, and nesting.
- **Sequence diagrams** — Support for sequence diagram type with participants, messages, activation boxes, fragments, and notes.
- **Type system** — User-defined types with attribute inheritance and built-in shape types (Rectangle, Oval, Actor, Boundary, Control, Entity, Interface, Component).
- **Layout engines** — Basic layout engine for component and sequence diagrams; Sugiyama layout engine for component diagrams.
- **SVG rendering** — Export diagrams to SVG with configurable styling.
- **Parser** — Full parsing pipeline: tokenizer, parser, desugaring, validation, and elaboration with structured error diagnostics (error codes, labeled spans, help text).
- **CLI** — `orrery` command-line tool for rendering `.orr` files to SVG with configurable output, logging, and TOML-based configuration.
- **Configuration** — Layered configuration via CLI flags, project-local files, and platform-specific config directories.
- **Dual licensing** — MIT OR Apache-2.0.

[Unreleased]: https://github.com/foadnh/orrery/compare/4b825dc642cb6eb9a060e54bf8d69288fbee49040...HEAD
