# Filament Diagram Language Specification

Filament is a domain-specific language designed for creating and rendering diagrams, with a focus on component and sequence diagrams. This specification documents the syntax, semantics, and features of the Filament language.

## 1. Introduction

Filament allows you to define diagrams using a text-based syntax, which is then parsed, processed, and rendered as SVG graphics. The language provides a flexible type system for customizing appearance and a simple way to express relationships between elements.

## 2. Basic Structure

A Filament document consists of a diagram declaration, optional type definitions, and diagram elements.

```
diagram <kind> [attributes...];
[type definitions...]
[elements...]
```

Whitespace is generally ignored, and comments can be added using Rust-style syntax (`// comment`).

## 3. Diagram Types

Filament supports two types of diagrams:

- **Component Diagrams** (`component`): For visualizing component structures and their relationships
- **Sequence Diagrams** (`sequence`): For visualizing interactions and message flows between participants

Example:
```
diagram component;
```

Diagrams can also include attributes to customize their behavior:

```
diagram component [layout_engine="force"];
```

## 4. Type System

### 4.1 Built-in Types

Filament provides two built-in shape types:

- `Rectangle`: A rectangular shape with customizable properties
- `Oval`: An elliptical shape with customizable properties

### 4.2 Type Definitions

Custom types can be defined by extending built-in types:

```
type <TypeName> = <BaseType> [attribute1="value1", attribute2="value2", ...];
```

**Naming Conventions:**
- Type names must use CamelCase (e.g., `Database`, `ApiGateway`)
- Type names must start with an uppercase letter

Example:
```
type Database = Rectangle [fill_color="lightblue", rounded="10", line_width="2"];
```

## 5. Elements

### 5.1 Components

Components are the basic building blocks of diagrams:

```
<element_name> [as "Display Name"]: <TypeName> [attribute1="value1", ...] { nested elements... };
```

**Naming Conventions:**
- Element names typically use snake_case (e.g., `user_service`, `data_layer`)
- Element names must begin with a letter
- Element names can contain alphanumeric characters and underscores
- Optional display names can be provided using the `as "Display Name"` syntax
- If no display name is provided, the element name is used as the display text

Example:
```
// With display name
frontend_app as "Frontend Application": Rectangle [fill_color="#e6f3ff"];
// Without display name (will display "user_database" text)
user_database: Database;
```

### 5.2 Relations

Relations define connections between components:

```
<source> <relation_type> [attribute1="value1", ...] <target> [: "label"];
```

Example:
```
frontend_app -> [color="blue", width="2"] user_database: "Stores data";
```

#### 5.2.1 Relation Types

Filament supports four relation types:

- Forward (`->`)
- Backward (`<-`)
- Bidirectional (`<->`)
- Plain (`-`)

## 6. Attributes

Attributes customize the appearance and behavior of elements:

### 6.1 Common Attributes

- `fill_color`: The background color of a shape (e.g., `"#ff0000"`, `"red"`, `"rgb(255,0,0)"`)
- `line_color`: The border color of a shape
- `line_width`: The thickness of lines/borders (numeric value)
- `rounded`: Rounding radius for rectangle corners (numeric value)
- `font_size`: Size of text labels (numeric value)

### 6.2 Relation-specific Attributes

- `color`: The line color of the relation
- `width`: The thickness of the relation line (numeric value)
- `style`: The style of the arrow line (values: `"straight"`, `"curved"`, or `"orthogonal"`, default is `"straight"`)

### 6.3 Relation Labels

Relations can optionally include text labels to describe their purpose or meaning:

```
<source> <relation_type> [attributes...] <target>: "Label text";
```

Labels are displayed above the relation line with a background for readability.

## 7. Nesting and Hierarchy

Components can contain other elements, creating a hierarchical structure:

```
parent_system: Rectangle {
    child_service1: Oval;
    child_service2: Rectangle;
    child_service1 -> child_service2;
};
```

Nested components are positioned within their parent container and maintain their relationships.

## 8. Identifiers and Naming Conventions

- Type identifiers must use CamelCase (e.g., `Database`, `UserService`)
- Element identifiers typically use snake_case (e.g., `auth_service`, `user_db`)
- Identifiers can include alphanumeric characters and underscores
- Nested identifiers use `::` for qualification (e.g., `parent_system::child_service1`)
- Identifiers must start with a letter
- Identifiers are case-sensitive

## 9. Layout Behavior

Filament supports multiple layout engines that can be specified using the `layout_engine` attribute in the diagram declaration:

```
diagram component [layout_engine="force"];
```

Available layout engines:

- `basic`: The default layout engine with simple positioning (available for both component and sequence diagrams)
- `force`: A force-directed layout engine for more organic component positioning (available for component diagrams)
- `sugiyama`: A hierarchical layout engine for layered diagrams (available for component diagrams)

### 9.1 Component Diagrams

- Components are automatically positioned based on their relationships
- Nested components are arranged within their parent container
- Sizes are automatically calculated based on content and text
- Margins and padding are automatically applied for readability
- The layout algorithm can be selected with the `layout_engine` attribute

### 9.2 Sequence Diagrams

- Participants (components) are arranged horizontally
- Messages (relations) are displayed as horizontal arrows between participants
- Time flows downward, with messages ordered as they appear in the source
- Lifelines extend from each participant throughout the diagram

## 10. Rendering Output

Filament diagrams are rendered as SVG files with the following characteristics:

- Components are rendered using their defined shape type
- Relations are rendered as lines with appropriate arrowheads
- Text labels are positioned appropriately
- Nested elements are visually contained within their parents
- Component boundaries adjust to fit their content

## 11. Complete Examples

### 11.1 Component Diagram Example

```
diagram component [layout_engine="force"];

type Database = Rectangle [fill_color="lightblue", rounded="10"];
type Service = Rectangle [fill_color="#e6f3ff"];
type Client = Oval [fill_color="#ffe6e6"];

end_user as "End User": Client;
backend_system as "Backend System": Service {
    auth_service as "Auth Service": Service;
    user_db: Database;
    auth_service -> user_db;
};
api_gateway: Service;

end_user -> api_gateway;
api_gateway -> backend_system;
api_gateway -> [style="curved", color="red"] end_user: "Response";
backend_system -> [style="orthogonal", color="green"] user_database: "Query";
```

### 11.2 Sequence Diagram Example

```
diagram sequence;

user_agent: Rectangle;
api_service: Rectangle;
data_store: Rectangle;

user_agent -> [color="blue"] api_service: "Request";
api_service -> [color="green"] data_store;
data_store -> [color="green"] api_service;
api_service -> [color="blue"] user_agent;
```

## 12. Error Handling

Filament provides error reporting for various issues:

- Syntax errors during parsing
- References to undefined components
- Invalid attribute values
- Invalid type references
- Other semantic errors

Each error includes a description to help locate and fix the issue.

## 13. Configuration File

Filament supports configuration through a TOML file that can specify default settings for diagram rendering.

### 13.1 Configuration File Format

The configuration file uses TOML syntax and supports the following settings:

```toml
# Layout engine configuration
[layout]
# Default layout engine for component diagrams (basic, force, sugiyama)
component = "sugiyama"
# Default layout engine for sequence diagrams (basic)
sequence = "basic"
```

Layout engine values are case-sensitive and must match the supported enum values exactly.

### 13.2 Layout Engine Values

The layout engine names in the configuration file are string representations of the internal enum values:

| String Value | Layout Engine Type | Supported Diagram Types       |
|--------------|-------------------|------------------------------|
| "basic"      | Basic layout      | Component, Sequence          |
| "force"      | Force-directed    | Component                    |
| "sugiyama"   | Hierarchical      | Component                    |

### 13.3 Layout Engines Priority

When determining which layout engine to use, Filament follows this priority order:

1. Explicit layout engine in diagram declaration (`layout_engine` attribute)
2. Default layout engine in configuration file
3. Built-in default (`basic`)

## 14. Command Line Usage

Filament diagrams can be rendered using the command line tool:

```
filament [--log-level=LEVEL] [-c|--config=CONFIG.toml] [-o|--output=FILE.svg] input_file.fil
```

Where:
- `--log-level`: Sets the logging verbosity (off, error, warn, info, debug, trace)
- `-c, --config`: Path to a TOML configuration file (optional)
- `-o, --output`: Specifies the output SVG file path (defaults to "out.svg")
- `input_file.fil`: The path to the Filament source file
