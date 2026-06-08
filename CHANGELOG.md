# Changelog

All notable changes to the Orrery project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Arrow routing moved from SVG export into layout engines** — Engines now compute path geometry and produce control points; the core draw layer only renders them. A new `SmartArrowPlacer` routes each relation by its `style`. ([#137](https://github.com/orreryworks/orrery/pull/137))
- **Simplified qualified import paths across workspace** — Replaced verbose `module::Type` qualified paths with direct imports, reordered module declarations before use statements per style conventions. ([#129](https://github.com/orreryworks/orrery/pull/129))
- **Simplified `calculate_message_endpoint_x` signature** — Removed the redundant `participant_id` parameter; the function now derives it internally from the participant component. ([#130](https://github.com/orreryworks/orrery/pull/130))
- **Sequence message endpoints use activation snapshots** — Messages capture each side's active `ActivationTiming` at event time and compute endpoint X from the snapshot, replacing the Y-range scan over activation boxes. ([#132](https://github.com/orreryworks/orrery/pull/132))

### Fixed

- **Text label background no longer double-padded** — The configured padding was applied twice, leaving the background visibly larger than the text it wraps. ([#133](https://github.com/orreryworks/orrery/pull/133))
- **Narrow shape labels now correctly identified as having no inner content** — Previously they were treated as embedded content, pushing the text to the top of the shape instead of its center. ([#134](https://github.com/orreryworks/orrery/pull/134))
- **Font sizes now rendered in points** — Glyphs were drawn about 25% smaller than the configured size, leaving label backgrounds visibly larger than the rendered text. Default sizes were lowered to preserve the previous visual sizing. ([#135](https://github.com/orreryworks/orrery/pull/135))

## [0.4.0] - 2026-05-23

### Added

- **Edge routing for parallel edges, reverse edges, and self-loops** — Multiple arrows between the same components now render as separate, visually distinguishable curves. Reverse arrows render on opposite sides, and self-referencing relations render as visible loops. ([#104](https://github.com/orreryworks/orrery/issues/104))

### Changed

- **BREAKING: Default `ArrowStyle` changed to `Curved`** — The default arrow style is now `Curved` (was `Straight`). `Curved` renders a straight line when no control points are provided, and follows bezier control points when they are. ([#106](https://github.com/orreryworks/orrery/issues/106))
- **BREAKING: Arrow rendering API accepts control points** — `ArrowDrawer::draw_arrow`, `ArrowWithText::render_to_layers`, and `ArrowWithTextDrawer::draw_arrow_with_text` now require an additional `control_points: &[Point]` parameter. Pass `&[]` to preserve previous behavior. ([#106](https://github.com/orreryworks/orrery/issues/106))
- **BREAKING: Arrow rendering API accepts an optional label position** — `ArrowWithText::render_to_layers` and `ArrowWithTextDrawer::draw_arrow_with_text` now require an additional `text_position_override: Option<Point>` parameter, and `PositionedArrowWithText` exposes a `with_text_position` builder. ([#117](https://github.com/orreryworks/orrery/issues/117))

### Fixed

- **Removed redundant `normalize_offset` on embedded diagram containers** — Container positions were double-shifted: once during component layout and again during layer composition. The redundant first shift displaced containers from their layout-assigned positions, causing overlap with siblings and cascading misalignment in multi-level nesting. ([#111](https://github.com/orreryworks/orrery/issues/111))

## [0.3.0] - 2026-05-03

### Added

- **Graphviz layout engine for component diagrams** — Component diagrams can now select Graphviz as their layout engine with `layout_engine="graphviz"`, delegating spatial positioning to Graphviz for more balanced placements and fewer relation crossings on non-trivial graphs. Gated behind the optional `graphviz` Cargo feature (disabled by default for library crates, enabled by default in the CLI). ([#88](https://github.com/orreryworks/orrery/issues/88))
- **Default arrow text background** — Relation labels now render with a semi-transparent white background (`rgba(255, 255, 255, 0.85)`) by default, making text readable when overlapping the arrow line. ([#101](https://github.com/orreryworks/orrery/issues/101))

## [0.2.0] - 2026-04-20

### Added

- **Namespaced import system** — Reuse type definitions across files with `import "path";` and `library;` file headers. Imports are namespace-qualified (e.g., `styles::Service`), support transitive chaining, and include circular dependency detection, diamond deduplication, and cross-file error reporting with import traces. ([#45](https://github.com/orreryworks/orrery/issues/45))
- **Glob import** — Bring all types from a file flat into the current scope with `import "path"::*;`, removing the need for namespace prefixes. Supports last-writer-wins override semantics and transitive re-exports. ([#46](https://github.com/orreryworks/orrery/issues/46))
- **Diagram embedding via import** — Reference an imported diagram file as component content with `embed <name>` syntax (e.g., `import "auth_flow"; auth_box: Rectangle embed auth_flow;`). ([#49](https://github.com/orreryworks/orrery/issues/49))
- **Import aliasing** — Override the derived namespace name with `import "path" as alias;`, so imported types are accessed as `alias::Type` instead of the path-derived namespace. ([#48](https://github.com/orreryworks/orrery/issues/48))
- **Text label background layering** — Text label backgrounds now render above all other diagram elements (arrows, fragments, lifelines, activations, etc.), with only the text itself on top. This ensures labels with a background always remain legible regardless of overlapping elements. ([#75](https://github.com/orreryworks/orrery/issues/75))

### Changed

- **BREAKING: Embedded diagram syntax** — Embedded diagrams now use `embed { diagram <kind>; ... };` instead of `embed diagram <kind> { ... };`. The diagram header (with its semicolon) moves inside the braces, making the embedded block structurally identical to a top-level file. ([#53](https://github.com/orreryworks/orrery/issues/53))

## [0.1.1] - 2026-03-11

### Added

- Fragment layout now accounts for vertical space consumed by the operation label header, section title guards, and bottom padding, preventing overlaps with subsequent elements. ([#36](https://github.com/orreryworks/orrery/issues/36))

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

[Unreleased]: https://github.com/orreryworks/orrery/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/orreryworks/orrery/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/orreryworks/orrery/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/orreryworks/orrery/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/orreryworks/orrery/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/orreryworks/orrery/releases/tag/v0.1.0
