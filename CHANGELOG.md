# Changelog

All notable changes to the Orrery project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Component default label displays full qualified path instead of component name ([#4](https://github.com/orreryworks/orrery/issues/4)). Refactored `Id` to split into `name` and `namespace`, so components without an explicit `as "..."` label now show only the final path segment.
- Activation box starts at next relation instead of preceding relation ([#11](https://github.com/orreryworks/orrery/issues/11)). Activation boxes now anchor to the last relation position rather than the current Y cursor, so they visually start at the triggering message and end at the last message within the block.

### Changed

- **MSRV** — Bumped minimum supported Rust version from 1.86 to 1.88 to allow stable `let` chains syntax. ([#6](https://github.com/orreryworks/orrery/issues/6))
- Sequence diagram vertical spacing is now based on actual element size plus configurable padding, instead of a fixed value for all events. ([#33](https://github.com/orreryworks/orrery/issues/33))

## [0.1.0] - 2026-02-25

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

[Unreleased]: https://github.com/orreryworks/orrery/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/orreryworks/orrery/releases/tag/v0.1.0
