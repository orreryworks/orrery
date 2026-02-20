# Orrery Language Roadmap

## Overview

This roadmap outlines the planned development path for the Orrery language and its ecosystem. Orrery is a domain-specific language for creating diagrams and visualizations with a focus on UML diagrams, sequence diagrams, and component diagrams.

The roadmap is organized into major feature categories, each containing specific improvements and new capabilities planned for future releases.

## Table of Contents

In sequence diagram, move max lifeline f32 to to lifeline property

### Core Components
- **[Language Core](#language-core)** - Core language features, syntax, and semantics
  - [Support for Importing Other .orr Files](#support-for-importing-other-orr-files)
  - [Add Support for Class Diagrams](#add-support-for-class-diagrams)
  - [Configurable Activation Box Definitions](#configurable-activation-box-definitions)
- **[AST](#ast)** - Parser improvements and error handling
  - [Multi Error Reporting](#multi-error-reporting)
  - [Empty/Unavailable Span](#emptyunavailable-span)
- **[Engines](#engines)** - Diagram generation and processing engines
  - [Fix Cross Level Relations in Component Diagram](#fix-cross-level-relations-in-component-diagram)
  - [Pre-compute Activation Box Associations for Messages](#pre-compute-activation-box-associations-for-messages)
- **[Type System](#type-system)** - Type checking and validation system
  - [Add Support for Prelude of Shapes](#add-support-for-prelude-of-shapes)
  - [Add Base Type Override Support](#add-base-type-override-support)
  - [Fix Scoping Types in graph::Graph](#fix-scoping-types-in-graphgraph)
  - [Stroke Style Validation](#stroke-style-validation)
- **[Rendering](#rendering)** - Visual output and drawing capabilities
  - [Adding More UML Shapes](#adding-more-uml-shapes)
  - [Relation-Triggered Activation](#relation-triggered-activation)
  - [Support Shapes with Custom Icons](#support-shapes-with-custom-icons)
  - [Custom Shape Definitions](#custom-shape-definitions)
  - [Animation Support](#animation-support)

### Tooling & Ecosystem
- **[Tooling](#tooling)** - Development tools and utilities
  - [fmt Feature for Formatting .orr Files](#fmt-feature-for-formatting-orr-files)
  - [Add Support for Multi Config Loading with Priority](#add-support-for-multi-config-loading-with-priority)
  - [Language Server Protocol (LSP)](#language-server-protocol-lsp)
- **[Integrations](#integrations)** - Editor extensions and IDE support
  - [Zed Extension](#zed-extension)
  - [VS Code Extension](#vs-code-extension)
  - [JetBrains Extension](#jetbrains-extension)

## Features

### Language Core

#### Support for Importing Other .orr Files

**Description**:
Add support for importing and using types and components from other Orrery files, enabling modular diagram development.

**Proposed Syntax**:
```orrery
import "common/types.orr" as common;
import "styles/buttons.orr" use ButtonStyle, PrimaryButton;

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
```orrery
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
```orrery
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

### AST

#### Multi Error Reporting

**Description**:
Enhance the error reporting system to collect and display multiple errors in a single compilation pass, rather than stopping at the first error encountered.

**Current Limitation**:
The compiler currently stops at the first error, requiring multiple compilation cycles to identify all issues in a file.

**Proposed Solution**:
```orrery
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

#### Empty/Unavailable Span

**Description**:
Implement a proper representation for empty or unavailable spans to fix incorrect behavior when combining spans with `Span::default()`.

**Current Problem**:
Currently, `Span::default()` creates a span at position 0 with zero length. When performing a union operation with another span, this incorrectly expands the result to include position 0, even though there is no actual content at that position. This leads to inaccurate span information in error reporting and diagnostics.

**Proposed Solution**:
Add an `Empty` or `Unavailable` variant to the `Span` type to explicitly represent the absence of a valid span. Update the `union` operation to handle this case correctly:
- Union of an empty span with any other span should return the non-empty span
- Union of two empty spans should return an empty span
- Update all usage sites that currently rely on `Span::default()` to use the new empty span representation

**Benefits**:
- Accurate span information in error messages
- Correct source location tracking throughout the compilation pipeline
- More predictable behavior when combining span information
- Clearer intent when a span is truly unavailable vs. a zero-length span at a specific position

**Implementation Considerations**:
- Audit all current uses of `Span::default()` in the codebase
- Update `Span::union()` logic to handle empty spans correctly
- Consider whether `Option<Span>` is a better alternative to an empty variant
- Ensure backward compatibility with existing span operations
- Update tests to cover empty span behavior

**References**:
- `src/ast/span.rs` - Current span implementation

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
```orrery
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
```orrery
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

#### Stroke Style Validation

**Description**:
Add validation for custom stroke style patterns to ensure they follow the correct dasharray format.

**Current Behavior**:
Custom stroke patterns (e.g., `style="5,3"` or `style="10,5,2,5"`) are currently accepted without validation. Invalid patterns are passed directly to SVG, which may result in rendering issues.

**Proposed Implementation**:
- Validate custom dash patterns during elaboration
- Accept formats: `"solid"`, `"dashed"`, `"dotted"`, or comma-separated numbers
- Reject invalid patterns with clear error messages
- Ensure at least 2 numbers in custom patterns (dash, gap)
- Verify all values are positive numbers

**Example Validation**:
```orrery
// Valid patterns
type Valid1 = Rectangle [stroke=[style="5,3"]];           // ✓ Simple pattern
type Valid2 = Rectangle [stroke=[style="10,5,2,5"]];      // ✓ Complex pattern
type Valid3 = Rectangle [stroke=[style="solid"]];         // ✓ Named style

// Invalid patterns (should error)
type Invalid1 = Rectangle [stroke=[style="5"]];           // ✗ Need at least 2 values
type Invalid2 = Rectangle [stroke=[style="abc,xyz"]];     // ✗ Non-numeric values
type Invalid3 = Rectangle [stroke=[style="-5,3"]];        // ✗ Negative values
```

**Benefits**:
- Earlier error detection
- Better error messages for users
- Prevents invalid SVG output
- Consistent behavior across all stroke contexts

**Implementation Considerations**:
- Add validation in stroke attribute extraction
- Maintain backward compatibility with existing valid patterns
- Clear documentation of supported formats

---

### Rendering

#### Adding More UML Shapes

**Description**:
Expand the built-in shape library with additional UML shapes to support comprehensive UML diagram creation.

**Proposed Shapes**:
- `Package`: UML package notation with folder-like appearance
- `Database`: Traditional database cylinder shape
- `Cloud`: Cloud computing representation
- `Queue`: Message queue representation
- `Process`: Hexagonal process shape
- `Decision`: Diamond decision shape
- `Document`: Document shape with curved bottom

**Example Usage**:
```orrery
auth_service: Package [fill_color="lightblue"];
user_note: Note [fill_color="yellow"];
main_db: Database [fill_color="green"];
```

**Benefits**:
- Comprehensive UML diagram support
- Reduced need for custom shape definitions
- Standard visual vocabulary

---

#### Relation-Triggered Activation

**Description**:
Add syntactic sugar that links an activation block directly to the message that triggers it. This provides an extremely concise way to express simple request-response flows by combining message sending with activation in a single statement.

**Proposed Syntax**:
```orrery
<source> -> <target>: "message" activate {
    ... // Target is active here
}
```

**Example**:
```orrery
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
```orrery
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
```orrery
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
```orrery
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


### Tooling

#### fmt Feature for Formatting .orr Files

**Description**:
Implement a code formatter for Orrery files to ensure consistent code style and improve readability.

**Proposed Usage**:
```bash
orrery fmt diagram.orr           # Format single file
orrery fmt src/                  # Format directory
orrery fmt --check diagram.orr   # Check formatting without changes
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
2. Project-specific config (`./orrery/config.toml`)
3. User config (`~/.config/orrery/config.toml`)
4. System config (`/etc/orrery/config.toml`)
5. Built-in defaults (lowest priority)

**Example Usage**:
```bash
orrery --config=project.toml --config=override.toml diagram.orr
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
Implement LSP support for Orrery to enable rich editor features like syntax highlighting, error checking, auto-completion, and go-to-definition.

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
Develop a Zed editor extension for Orrery with syntax highlighting, error reporting, and basic language features.

**Features**:
- Syntax highlighting for .orr files
- Error underlining and hover information
- Basic auto-completion for built-in types
- Integration with Orrery compiler

**Benefits**:
- Native editor support for Zed users
- Improved development experience
- Real-time feedback while editing

---

#### VS Code Extension

**Description**:
Create a comprehensive VS Code extension providing rich language support for Orrery diagrams.

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
Develop a JetBrains plugin supporting Orrery across IntelliJ IDEA, WebStorm, and other JetBrains IDEs.

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
