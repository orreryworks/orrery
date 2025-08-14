# Filament Language Roadmap

## Overview

This roadmap outlines the planned development path for the Filament language and its ecosystem. Filament is a domain-specific language for creating diagrams and visualizations with a focus on UML diagrams, sequence diagrams, and component diagrams.

The roadmap is organized into major feature categories, each containing specific improvements and new capabilities planned for future releases.

## Table of Contents

### Core Components
- **[Language Core](#language-core)** - Core language features, syntax, and semantics
  - [Named Types for Nested Attributes](#named-types-for-nested-attributes)
  - [Support for Importing Other .fil Files](#support-for-importing-other-fil-files)
  - [Add Support for Class Diagrams](#add-support-for-class-diagrams)
  - [Configurable Activation Box Definitions](#configurable-activation-box-definitions)
  - [Configurable Lifeline Definitions from Sequence Attributes](#configurable-lifeline-definitions-from-sequence-attributes)
- **[AST](#ast)** - Parser improvements and error handling
  - [Multi Error Reporting](#multi-error-reporting)
  - [Improve Error Messages](#improve-error-messages)
- **[Engines](#engines)** - Diagram generation and processing engines
  - [Fix Cross Level Relations in Component Diagram](#fix-cross-level-relations-in-component-diagram)
  - [Pre-compute Activation Box Associations for Messages](#pre-compute-activation-box-associations-for-messages)
- **[Type System](#type-system)** - Type checking and validation system
  - [Add Support for Prelude of Shapes](#add-support-for-prelude-of-shapes)
  - [Add Base Type Override Support](#add-base-type-override-support)
  - [Fix Scoping Types in graph::Graph](#fix-scoping-types-in-graphgraph)
- **[Rendering](#rendering)** - Visual output and drawing capabilities
  - [Adding More UML Shapes](#adding-more-uml-shapes)
  - [Explicit Activate/Deactivate Statements](#explicit-activatedeactivate-statements)
  - [Relation-Triggered Activation](#relation-triggered-activation)
  - [Support Shapes with Custom Icons](#support-shapes-with-custom-icons)
  - [Custom Shape Definitions](#custom-shape-definitions)
  - [Animation Support](#animation-support)
  - [Alpha Transparency Support](#alpha-transparency-support)

### Tooling & Ecosystem
- **[Tooling](#tooling)** - Development tools and utilities
  - [fmt Feature for Formatting .fil Files](#fmt-feature-for-formatting-fil-files)
  - [Add Support for Multi Config Loading with Priority](#add-support-for-multi-config-loading-with-priority)
  - [Language Server Protocol (LSP)](#language-server-protocol-lsp)
- **[Integrations](#integrations)** - Editor extensions and IDE support
  - [Zed Extension](#zed-extension)
  - [VS Code Extension](#vs-code-extension)
  - [JetBrains Extension](#jetbrains-extension)

## Features

### Language Core

#### Named Types for Nested Attributes

**Description**:
Allow creation of named types for nested attribute groups, enabling reusable attribute collections that can be referenced by name in type definitions.

**Current Limitation**:
Currently, nested attributes must be defined inline, leading to duplication and reduced maintainability:

```filament
type Button = Rectangle [
    fill_color="blue",
    text=[font_size=16, font_family="Arial", background_color="white", padding=8.0]
];

type ImportantButton = Rectangle [
    fill_color="red",
    text=[font_size=16, font_family="Arial", background_color="white", padding=8.0]  // Duplication
];
```

**Proposed Solution**:
Introduce named types for nested attribute groups:

```filament
// Define a reusable text style
type StandardText = Text [font_size=16, font_family="Arial", background_color="white", padding=8.0];

// Define a highlighted text style extending the standard
type ImportantText = StandardText [background_color="yellow"];

// Use named text types in component definitions
type Button = Rectangle [
    fill_color="blue",
    text=StandardText
];

type ImportantButton = Rectangle [
    fill_color="red",
    text=ImportantText
];
```

**Benefits**:
- Reduces code duplication
- Improves maintainability of style definitions
- Enables consistent styling across components
- Supports hierarchical style inheritance

**Implementation Considerations**:
- Parser modifications to handle `Type [attributes]` syntax for nested attribute types
- Type resolver updates to handle nested type references
- Error handling for circular dependencies in nested types
- Documentation updates for new syntax patterns

---

#### Support for Importing Other .fil Files

**Description**:
Add support for importing and using types and components from other Filament files, enabling modular diagram development.

**Proposed Syntax**:
```filament
import "common/types.fil" as common;
import "styles/buttons.fil" use ButtonStyle, PrimaryButton;

// Use imported types
component: common.DatabaseType;
button: ButtonStyle;
```

**Benefits**:
- Code reusability across projects
- Modular diagram organization
- Shared style libraries
- Team collaboration improvements

**Implementation Considerations**:
- Module resolution and path handling
- Circular dependency detection
- Namespace management
- Error reporting across file boundaries

---

#### Add Support for Class Diagrams

**Description**:
Introduce a new diagram type specifically designed for UML class diagrams with support for classes, interfaces, inheritance, and associations.

**Proposed Syntax**:
```filament
diagram class;

type Class = Rectangle [
    sections=[attributes, methods],
    visibility_icons=true
];

UserService: Class {
    attributes: [
        "- id: String",
        "+ name: String",
        "# email: String"
    ];
    methods: [
        "+ getName(): String",
        "+ setEmail(email: String): void"
    ];
};

UserService -> [inheritance] BaseService;
UserService -> [association] Database: "uses";
```

**Benefits**:
- Dedicated support for object-oriented design
- Built-in UML class diagram conventions
- Proper inheritance and association representations
- Integration with existing type system

---

#### Configurable Activation Box Definitions

**Description**:
Make activation box styling configurable from the language syntax, allowing users to customize activation box appearance using attribute syntax similar to other visual elements.

**Current Implementation**:
- Activation boxes use hardcoded default styling (white fill with black border)
- ActivationBoxDefinition is created with default values only
- No language-level customization available for activation box appearance

**Proposed Implementation**:
- Add attribute support to activate block syntax: `activate component [fill_color="red", line_color="blue"] { ... };`
- Extend ActivationBoxDefinition to support configurable attributes:
  - `fill_color`: Background color of activation boxes
  - `line_color`: Border color of activation boxes
  - `line_width`: Border thickness of activation boxes
  - `rounded`: Corner radius for rounded activation boxes
- Update parser to handle optional attribute specifications in activate blocks
- Integrate with existing attribute system for type safety and validation

**Benefits**:
- Enhanced visual customization for sequence diagrams
- Consistent syntax with other language elements
- Better visual distinction between different types of activations
- Improved diagram aesthetics and readability
- Maintains backward compatibility with existing syntax

**Example Usage**:
```filament
diagram sequence;

user: Rectangle;
server: Rectangle;

// Default styling
activate user {
    user -> server: "Request";
};

// Custom styling
activate server [fill_color="lightblue", line_color="darkblue", line_width=2.0] {
    server -> user: "Response";
};
```

---

#### Configurable Lifeline Definitions from Sequence Attributes

**Description**:
Enable configuring lifeline visual and layout properties directly from sequence diagram attributes. Lifelines are the vertical lines that represent participant existence over time. Today, their appearance and spacing are fixed; this feature makes them first-class configurable elements to match different visual styles and layout needs.

**Current Limitation**:
- Lifeline line color, width, and style are not customizable via the language
- Horizontal spacing between participants is fixed or globally internal
- Top/bottom margins for lifelines cannot be tuned per diagram

**Proposed Implementation**:
- Add a `lifeline=[...]` nested attribute group to sequence diagram declarations:
  - `line_color` (string): Color of lifeline (e.g., `"#bbbbbb"`, `"gray"`)
  - `line_width` (float): Stroke width of lifeline (e.g., `1.0`, `1.5`)
  - `line_style` (string): Line style, e.g., `"solid"` or `"dashed"`
  - `spacing` (float): Horizontal distance between adjacent participants
  - `top_padding` (float): Vertical padding before the first message
  - `bottom_padding` (float): Vertical padding after the last message
- Parsing:
  - Extend the sequence diagram parser to accept an optional `lifeline=[...]` attribute group on the `diagram sequence [...]` declaration
- Engine integration:
  - Use `spacing` during participant layout
  - Apply `line_color`, `line_width`, `line_style` when drawing lifelines
  - Respect `top_padding` and `bottom_padding` when computing the diagram height and lifeline lengths
- Validation:
  - Enforce value types (strings for colors/styles, floats for numeric values)
  - Provide clear errors for invalid `line_style` values

**Benefits**:
- Consistent, theme-able lifeline styling across sequence diagrams
- Better control over density and readability via adjustable spacing
- Professional look that matches organizational branding or documentation style
- Backward-compatible: defaults preserve current behavior when `lifeline` is omitted

**Example Usage**:
```filament
diagram sequence [
    lifeline=[
        line_color="#bbbbbb",
        line_width=1.5,
        line_style="dashed",
        spacing=160.0,
        top_padding=24.0,
        bottom_padding=16.0
    ]
];

client: Rectangle;
service: Rectangle;
db: Rectangle;

client -> service: "Request";
service -> db: "Query";
db -> service: "Result";
service -> client: "Response";
```

---

### AST

#### Multi Error Reporting

**Description**:
Enhance the error reporting system to collect and display multiple errors in a single compilation pass, rather than stopping at the first error encountered.

**Current Limitation**:
The compiler currently stops at the first error, requiring multiple compilation cycles to identify all issues in a file.

**Proposed Solution**:
```filament
// Instead of stopping at first error, report all issues:
// Error 1: Missing semicolon at line 5
// Error 2: Undefined type 'Rectangl' at line 8
// Error 3: Invalid attribute 'colour' at line 12
```

**Benefits**:
- Faster development cycle by showing all errors at once
- Better developer experience
- Reduced compilation iterations

**Implementation Considerations**:
- Error recovery mechanisms in parser
- Error collection and batching system
- Maintaining accurate source locations across multiple errors

---

#### Improve Error Messages

**Description**:
Enhance error messages with more context, better suggestions, and improved formatting for common mistakes.

**Proposed Improvements**:
- Better suggestions for typos in type names and attributes
- More context around the error location
- Common fix suggestions based on error patterns
- Better highlighting of problematic code sections

**Example**:
```
Current: "Parse error: unexpected token"
Improved: "Parse error: expected ';' after component definition
   Did you mean to terminate this component declaration?
   Similar issue: Missing semicolon is a common syntax error"
```

**Benefits**:
- Reduced debugging time
- Better learning experience for new users
- More actionable error guidance

---

### Engines

#### Fix Cross Level Relations in Component Diagram

**Description**:
Resolve issues with relations that cross different nesting levels in component diagrams, ensuring proper visual representation and layout.

**Current Issue**:
Relations between components at different nesting levels may not render correctly or cause layout problems.

**Proposed Solution**:
- Improve relation routing algorithms for cross-level connections
- Better visual representation of hierarchical relations
- Proper z-ordering and intersection handling

**Benefits**:
- More flexible diagram structures
- Better support for complex system architectures
- Improved visual clarity

#### Pre-compute Activation Box Associations for Messages

**Description**:
Optimize sequence diagram engine performance by pre-computing activation box associations during layout instead of searching for them during each message render operation.

**Current Implementation**:
The sequence engine searches for active activation boxes for each message during the render step, which involves:
- Iterating through all activation boxes for each message
- Checking if activation boxes are active at the message's Y position
- Determining intersection points for each message independently

**Proposed Implementation**:
- Add two optional fields to the `Message` struct: `source_activation_box` and `target_activation_box`
- Modify `calculate_activation_boxes()` function in the sequence engine to populate these fields during layout calculation
- Update message rendering to use pre-computed activation box references directly

**Technical Details**:
```rust
pub struct Message {
    pub source_index: usize,
    pub target_index: usize,
    pub y_position: f32,
    arrow_with_text: draw::ArrowWithText,
    // New fields for sequence engine optimization
    pub source_activation_box: Option<usize>, // Index into activation boxes array
    pub target_activation_box: Option<usize>, // Index into activation boxes array
}
```

**Benefits**:
- **Significant performance improvement**: Eliminates O(n×m) search complexity (n messages × m activation boxes)
- **Reduced computational overhead**: No repeated searches during sequence engine processing
- **Better cache locality**: Pre-computed references improve memory access patterns
- **Simplified render logic**: Direct activation box access instead of complex queries
- **Maintains accuracy**: Same visual results with better performance

**Implementation Strategy**:
1. Extend `Message` struct with optional activation box indices
2. Modify sequence engine's `calculate_activation_boxes()` to build activation box associations during layout
3. Update message rendering to use pre-computed associations from the sequence engine
4. Maintain backward compatibility for messages without activation boxes

**Impact**:
This sequence engine optimization is particularly beneficial for complex sequence diagrams with many messages and nested activation boxes, where the current search-based approach becomes a performance bottleneck.

---

### Type System

#### Add Support for Prelude of Shapes

**Description**:
Introduce a prelude system that automatically imports common shapes and types, reducing boilerplate in diagram files.

**Proposed Implementation**:
```filament
// Automatically available without import:
// Rectangle, Oval, Component, etc.
// Common type aliases and utility types

diagram component; // No need to import basic shapes

component: Rectangle; // Automatically available
```

**Benefits**:
- Reduced boilerplate in simple diagrams
- Better beginner experience
- Consistent baseline functionality

**Implementation Considerations**:
- Configurable prelude content
- Override mechanisms for custom definitions
- Documentation of prelude contents

---

#### Add Base Type Override Support

**Description**:
Allow overriding attributes from base types when defining custom types, providing more flexible type composition.

**Proposed Syntax**:
```filament
type BaseButton = Rectangle [fill_color="blue", line_width=1.0];

// Override base attributes
type RedButton = BaseButton [fill_color="red" override]; // Explicitly override
type ThickRedButton = BaseButton [fill_color="red", line_width=3.0]; // Implicit override
```

**Benefits**:
- More flexible type inheritance
- Explicit control over attribute overriding
- Better type composition patterns

---

#### Fix Scoping Types in graph::Graph

**Description**:
Improve the scoping and type handling within the graph::Graph structure to provide better type resolution and namespace management.

**Current Issues**:
- Type scoping may not be properly handled in complex nested scenarios
- Graph type resolution could be improved for better error reporting and validation
- Type information propagation through the graph structure needs refinement

**Proposed Implementation**:
- Enhance type scope tracking within Graph structure
- Improve type resolution algorithms for nested components
- Better integration between AST type information and graph representation
- Cleaner separation of type concerns in graph processing

**Benefits**:
- More accurate type checking and validation
- Better error messages for type-related issues
- Improved performance in type resolution
- Cleaner graph processing pipeline

---

### Rendering

#### Adding More UML Shapes

**Description**:
Expand the built-in shape library with additional UML shapes to support comprehensive UML diagram creation.

**Proposed Shapes**:
- `Package`: UML package notation with folder-like appearance
- `Note`: UML note/comment shape with folded corner
- `Database`: Traditional database cylinder shape
- `Cloud`: Cloud computing representation
- `Queue`: Message queue representation
- `Process`: Hexagonal process shape
- `Decision`: Diamond decision shape
- `Document`: Document shape with curved bottom

**Example Usage**:
```filament
auth_service: Package [fill_color="lightblue"];
user_note: Note [fill_color="yellow"];
main_db: Database [fill_color="green"];
```

**Benefits**:
- Comprehensive UML diagram support
- Reduced need for custom shape definitions
- Standard visual vocabulary

---

#### Explicit Activate/Deactivate Statements

**Description**:
Add support for explicit `activate` and `deactivate` statements in sequence diagrams to provide granular control over activation boxes (lifeline spans). This approach is ideal for modeling asynchronous workflows or complex interactions where activation and deactivation events are not tightly scoped.

**Proposed Syntax**:
```filament
activate <component_name>;
... // Component is active
deactivate <component_name>;
```

**Example**:
```filament
diagram sequence;
user: Rectangle;
server: Rectangle;

// User sends a job and immediately deactivates
activate user;
user -> server: "Process this job";
deactivate user;

// Server activates later to perform the work independently
activate server;
server -> server: "Working on job...";
deactivate server;
```

**Benefits**:
- Most granular control over activation timing
- Perfect for asynchronous communication patterns
- Supports "fire-and-forget" message flows
- Handles complex scenarios where activation scope isn't confined to a single block

**Best Use Cases**:
- Asynchronous communication
- "Fire-and-forget" messages
- Complex scenarios with independent activation lifecycles

---

#### Relation-Triggered Activation

**Description**:
Add syntactic sugar that links an activation block directly to the message that triggers it. This provides an extremely concise way to express simple request-response flows by combining message sending with activation in a single statement.

**Proposed Syntax**:
```filament
<source> -> <target>: "message" activate {
    ... // Target is active here
}
```

**Example**:
```filament
diagram sequence;
user: Rectangle;
server: Rectangle;

// This one line replaces multiple lines from other approaches
user -> server: "Get user data" activate {
    server -> server: "Query database";
    server -> user: "Here is your data";
}
```

**Benefits**:
- Most concise syntax for simple request-response patterns
- Intuitive linking of trigger message to activation
- Reduces boilerplate code
- Clear cause-and-effect relationship

**Implementation Note**:
This can be implemented as syntactic sugar that transforms into a standard message followed by an `activate` block, making it compatible with the scoped activation approach.

**Best Use Cases**:
- Simple request-response flows
- Method calls with immediate processing
- Reducing syntax overhead for common patterns

---

#### Support Shapes with Custom Icons

**Description**:
Allow shapes to include custom icons or symbols, either from icon libraries or custom SVG definitions.

**Proposed Syntax**:
```filament
type DatabaseService = Rectangle [
    fill_color="lightblue",
    icon="database", // From built-in icon set
    icon_position="top-left"
];

type CustomService = Rectangle [
    fill_color="green",
    icon=svg("path/to/custom-icon.svg"),
    icon_size=16.0
];
```

**Benefits**:
- More visually distinctive components
- Professional-looking diagrams
- Better semantic representation

---

#### Custom Shape Definitions

**Description**:
Allow users to define custom shapes using SVG path notation or simple geometric primitives.

**Example**:
```filament
shape CustomShape = path "M10,10 L20,20 L10,30 Z" [
    viewBox="0 0 40 40",
    fill_color="blue"
];

type MyComponent = CustomShape [fill_color="red"];
```

---

#### Animation Support

**Description**:
Add support for basic animations in rendered SVG output.

**Example**:
```filament
type PulsingComponent = Rectangle [
    fill_color="blue",
    animation=[
        type="pulse",
        duration=2.0,
        iterations="infinite"
    ]
];
```

---

#### Alpha Transparency Support

**Description**:
Add comprehensive support for alpha transparency (opacity) in all color attributes throughout Filament, enabling semi-transparent colors for more sophisticated visual designs.

**Current Limitation**:
Currently, alpha transparency is only partially supported in some color contexts. A comprehensive system is needed to support RGBA colors consistently across all color attributes.

**Proposed Implementation**:
```filament
type SemiTransparentBox = Rectangle [
    fill_color="rgba(255, 0, 0, 0.5)",     // Semi-transparent red background
    line_color="rgba(0, 0, 255, 0.8)",     // Semi-transparent blue border
    text=[
        color="rgba(255, 255, 255, 0.9)",   // Semi-transparent white text
        background_color="rgba(0, 0, 0, 0.3)" // Semi-transparent black text background
    ]
];

// Relations with transparency
component1 -> [color="rgba(0, 255, 0, 0.6)"] component2: "Semi-transparent relation";

// Diagram background with transparency
diagram component [background_color="rgba(240, 240, 240, 0.8)"];
```

**Benefits**:
- Enhanced visual design capabilities
- Support for layered visual effects
- Better integration with complex backgrounds
- Professional-looking semi-transparent overlays
- Consistent alpha support across all color attributes

**Implementation Considerations**:
- Ensure RGBA support in all color parsing contexts
- Maintain backward compatibility with existing color formats
- Proper SVG opacity attribute generation
- Documentation updates for RGBA syntax
- Error handling for invalid alpha values (must be 0.0-1.0)

---


### Tooling

#### fmt Feature for Formatting .fil Files

**Description**:
Implement a code formatter for Filament files to ensure consistent code style and improve readability.

**Proposed Usage**:
```bash
filament fmt diagram.fil           # Format single file
filament fmt src/                  # Format directory
filament fmt --check diagram.fil   # Check formatting without changes
```

**Features**:
- Consistent indentation and spacing
- Proper alignment of attributes
- Configurable formatting rules
- Integration with editors and CI/CD

**Benefits**:
- Consistent code style across projects
- Reduced manual formatting effort
- Better collaboration and code reviews

**Implementation Considerations**:
- Preserve comments and spacing where meaningful
- Configurable formatting preferences
- Integration with existing toolchain

---

#### Add Support for Multi Config Loading with Priority

**Description**:
Enhance the configuration system to support loading multiple configuration files with a defined priority system.

**Proposed Priority Order**:
1. Command-line arguments (highest priority)
2. Project-specific config (`./filament/config.toml`)
3. User config (`~/.config/filament/config.toml`)
4. System config (`/etc/filament/config.toml`)
5. Built-in defaults (lowest priority)

**Example Usage**:
```bash
filament --config=project.toml --config=override.toml diagram.fil
```

**Benefits**:
- Flexible configuration management
- Environment-specific overrides
- Team and personal preference support

**Implementation Considerations**:
- Configuration merging strategies
- Override conflict resolution
- Clear precedence documentation

---

#### Language Server Protocol (LSP)

**Description**:
Implement LSP support for Filament to enable rich editor features like syntax highlighting, error checking, auto-completion, and go-to-definition.

**Features**:
- Syntax highlighting
- Real-time error detection
- Auto-completion for types and attributes
- Hover information for types and elements
- Go-to-definition for type references
- Rename refactoring

---

### Integrations

#### Zed Extension

**Description**:
Develop a Zed editor extension for Filament with syntax highlighting, error reporting, and basic language features.

**Features**:
- Syntax highlighting for .fil files
- Error underlining and hover information
- Basic auto-completion for built-in types
- Integration with Filament compiler

**Benefits**:
- Native editor support for Zed users
- Improved development experience
- Real-time feedback while editing

---

#### VS Code Extension

**Description**:
Create a comprehensive VS Code extension providing rich language support for Filament diagrams.

**Features**:
- Advanced syntax highlighting
- IntelliSense auto-completion
- Error diagnostics and quick fixes
- Live preview of diagrams
- Snippet support for common patterns
- Integration with workspace settings

**Benefits**:
- Full-featured IDE experience
- Large user base support
- Rich ecosystem integration

---

#### JetBrains Extension

**Description**:
Develop a JetBrains plugin supporting Filament across IntelliJ IDEA, WebStorm, and other JetBrains IDEs.

**Features**:
- Smart code completion
- Advanced refactoring capabilities
- Integrated debugging support
- Code inspection and quick fixes
- Version control integration

**Benefits**:
- Enterprise development environment support
- Advanced IDE features
- Professional workflow integration
