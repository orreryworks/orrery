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

Filament uses a unified Type System for defining and applying types to all language constructs. The system distinguishes between **declarations** (`:`) for creating named instances and **invocations** (`@`) for performing actions with types.

**For complete Type System documentation, see:** [Type System Specification](type_system.md)

### 4.1 Quick Reference: Built-in Base Types

**Component Types:**
- `Rectangle`, `Oval`, `Component` - Accept nested elements
- `Boundary`, `Actor`, `Entity`, `Control`, `Interface` - Content-free (no nesting)

**Relation Type:**
- `Arrow` - Base type for connections

**Construct Types:**
- `Note` - Annotations
- `Activate` - Activation blocks
- `Fragment` - Fragment blocks

**Attribute Group Types:**
- `Stroke`, `Text` - Reusable attribute sets

### 4.2 Basic Type Declaration

```
type TypeName = BaseType[attributes...];
```

**Naming Conventions:**
- Type names must use CamelCase (e.g., `Database`, `ApiGateway`)
- Type names must start with an uppercase letter

Example:
```
type Database = Rectangle [fill_color="lightblue", rounded=10, stroke=[width=2.0]];
type RedArrow = Arrow [stroke=[color="red"]];
type ThickRedArrow = RedArrow [stroke=[width=3.0]];
```
```

## 5. Literal Values and Data Types

Filament supports two primary data types for attribute values: string literals and float literals. For comprehensive documentation on syntax, usage, and examples, see:

**[Literal Values and Data Types Specification](literal_values.md)**

## 6. Elements

### 6.1 Components

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

Diagrams can have a background color specified as an attribute:
```
// Diagram with a light blue background
diagram component [background_color="#e6f3ff"];
```

### 6.2 Relations

Relations define connections between components using the following syntax:

```
<source> <relation_type> @TypeName [attributes...] <target> : "label";
```

Where:
- `<source>` and `<target>` are component identifiers
- `<relation_type>` is one of the four relation types (see below)
- `@TypeName` applies a type to the relation (optional - defaults to `@Arrow`)
- `[attributes...]` are optional attributes that override type attributes
- `: "label"` is an optional text label displayed on the relation

**For complete Type System documentation, see:** [Type System Specification](type_system.md)

#### 6.2.1 Relation Types

Filament supports four relation types:

- **Forward** (`->`) - Arrow pointing from source to target
- **Backward** (`<-`) - Arrow pointing from target to source
- **Bidirectional** (`<->`) - Arrows pointing in both directions
- **Plain** (`-`) - Simple line with no arrowheads

#### 6.2.2 Examples

**Type Definitions:**
```
type DashedArrow = Arrow[stroke=[style="dashed"]];
type ThickArrow = Arrow[stroke=[width=3.0, color="blue"]];
```

**Usage:**
```
// Sugar syntax (uses default @Arrow)
app -> database;
app -> database: "Query";

// Named TypeSpec
client -> @DashedArrow server: "Request";

// Anonymous TypeSpec (extending base type)
server -> @Arrow[color="red"] database: "Query";

// Anonymous TypeSpec (extending named type)
server -> @DashedArrow[color="blue"] client: "Response";
```

**Syntactic Sugar:**

When `@TypeSpec` is omitted, it defaults to `@Arrow`:
```
client -> server;           // Equivalent to: client -> @Arrow server;
client -> [color="red"] server;  // Equivalent to: client -> @Arrow [color="red"] server;
```

### 6.3 Activation (Blocks and Explicit Statements)

Activation defines periods when a component is active (also known as "focus of control") in sequence diagrams. Activation can be written in two interchangeable syntaxes that are fully equivalent: an explicit form using standalone statements, and a block form that provides a clearer lexical scope. Internally, block syntax is syntactic sugar that is desugared into explicit statements during compilation.

Preferred style: use block syntax whenever a clear lexical scope exists (it is more readable and self‑documenting). Use explicit statements when activation spans are non‑contiguous or intentionally asynchronous.

**Syntax (two equivalent forms):**

1) Explicit statements
```
activate @TypeName [attributes...] <component>;
deactivate <component_name>;
```

2) Block (sugar for explicit)
```
activate @TypeName [attributes...] <component> {
    // Elements active during this period
    // Can include relations, nested components, or other activate blocks
};
```

Where:
- `@TypeName` is optional (defaults to `@Activate`)
- `[attributes...]` creates an anonymous type
- `<component>` is the component identifier

**For complete Type System documentation, see:** [Type System Specification](type_system.md)

**Key Properties:**
- **Sequence diagrams only**: Activation is supported only in sequence diagrams
- **Temporal grouping**: Activation groups events in time; it does not create component namespaces
- **Visual representation**: Rendered as white rectangles with black borders on lifelines
- **Nestable**: Nested activation is supported (both statement and block forms)
- **Coexistence**: Both forms can be mixed; block form is preferred when a lexical scope exists

#### 6.3.1 Basic Usage (Block)

```
diagram sequence;

user: Rectangle;
server: Rectangle;

activate user {
    user -> server: "Request data";
    server -> user: "Response data";
};
```

#### 6.3.2 Nested Activation (Block)

```
diagram sequence;

client: Rectangle;
server: Rectangle;
database: Rectangle;

activate client {
    client -> server: "Initial request";

    activate server {
        server -> database: "Query data";
        database -> server: "Return results";
    };

    server -> client: "Final response";
};
```

#### 6.3.3 Custom Activation Types

```
diagram sequence;

type CriticalActivation = Activate[fill_color="red", stroke=[color="red", width=2.0]];

user: Rectangle;
service: Rectangle;
database: Rectangle;

// Named TypeSpec
activate @CriticalActivation user {
    user -> service: "Critical request";
    
    // Anonymous TypeSpec
    activate [fill_color="blue)"] service {
        service -> database: "Query";
    };
};
```

**Syntactic Sugar:**

When `@TypeName` is omitted, it defaults to `@Activate`:
```
activate user { };              // Equivalent to: activate @Activate user { };
activate [fill_color="red"] user { };  // Equivalent to: activate @Activate [fill_color="red"] user { };
```

#### 6.3.4 Scoping Behavior

**Important**: Activate blocks in sequence diagrams do NOT create component namespace scopes. Unlike component diagrams where `{}` creates nested scopes, activate blocks are purely for temporal grouping:

- ✅ **Correct**: `user -> server` (maintains flat naming)
- ❌ **Incorrect**: `user::server` (no namespace scoping in sequence diagrams)

This ensures activate blocks serve their purpose as temporal grouping constructs rather than hierarchical component organization.

#### 6.3.5 Diagram Type Restrictions

Activate blocks are only supported in sequence diagrams:

```
// ✅ Valid: Activate blocks in sequence diagram
diagram sequence;
activate user { user -> server; };

// ❌ Invalid: Activate blocks not allowed in component diagrams
diagram component;
activate user { user -> server; }; // ERROR: Not supported in component diagrams
```

#### 6.3.6 Explicit Usage (Statements)

Explicit activation statements provide granular control over lifeline activation timing and coexist with the block form. They are ideal for asynchronous workflows or cases where activation scope is not confined to a single block.

Syntax:
```
activate <component_name>;
deactivate <component_name>;
```

Notes:
- Preferred: Use the block form when a clear lexical scope exists; it provides better clarity and groups related interactions
- Supported only in sequence diagrams
- Component names can be nested identifiers (e.g., `parent::child`)
- Statements must appear in valid pairs for each component within a scope (activate first, then deactivate)
- Nesting is allowed (multiple activates before matching deactivates)
- Block form remains supported and is desugared to explicit statements internally

Example (explicit statements):
```
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

Equivalent using the block form (sugar):
```
diagram sequence;
user: Rectangle;
server: Rectangle;

activate user {
    user -> server: "Process this job";
};

activate server {
    server -> server: "Working on job...";
};
```

Desugaring:
- The compiler rewrites:
  - `activate <component> { ... };`
  - into `activate <component>; ... deactivate <component>;` preserving order and spans
- After desugaring, later phases (validation, elaboration, graph) operate only on explicit `activate`/`deactivate` statements
- Any `ActivateBlock` reaching elaboration is considered unreachable (internal compiler invariant)

Validation:
- A syntax-level validation pass ensures activation pairs are balanced and correctly ordered
- Semantic checks (diagram kind, component existence) occur during elaboration

### 6.4 Fragment Blocks

Fragments group related interactions in sequence diagrams into labeled sections. They help structure complex message flows, illustrate alternatives, and provide hierarchical organization.

#### 6.4.1 Syntax

```
fragment @TypeName [attributes...] "operation" {
    section "title" {
        // sequence elements...
        // Valid: component definitions, relations, activate blocks, nested fragments
    };
    // one or more sections
};
```

Where:
- `@TypeName` is optional (defaults to `@Fragment`)
- `[attributes...]` creates an anonymous type
- `"operation"` is a required string literal
- At least one section is required
- Section titles are optional; if present, they must be string literals
- Each section must end with a semicolon
- The fragment block must end with a semicolon

**For complete Type System documentation, see:** [Type System Specification](type_system.md)

#### 6.4.2 Fragment Attributes

Fragments support the following attributes to customize their appearance:

**Available Attributes:**

- `border_stroke=[...]`: Border styling for the fragment box (stroke attributes)
- `separator_stroke=[...]`: Styling for section separator lines (stroke attributes)
- `background_color`: Background color for the entire fragment (string color value)
- `content_padding`: Padding around fragment content (float value)
- `operation_label_text=[...]`: Text styling for the operation label (e.g., "alt", "loop") (text attributes)
- `section_title_text=[...]`: Text styling for section titles (text attributes)

**Examples:**

Basic fragment with custom colors:
```
fragment [background_color="lightyellow", border_stroke=[color="orange"]] "Authentication" {
    section "success" {
        client -> server: "Login successful";
    };
};
```

Fragment with custom text styling:
```
fragment [operation_label_text=[font_size=14, color="blue"]] "Process" {
    section "step 1" {
        server -> database: "Query";
    };
};
```

Fragment with separate operation and section title styling:
```
fragment [
    operation_label_text=[font_size=14, color="darkblue"],
    section_title_text=[font_size=12, color="gray"]
] "Data Flow" {
    section "initialization" {
        client -> server: "Initialize";
    };
    section "processing" {
        server -> database: "Process data";
    };
};
```

Using sugar syntax with attributes:
```
alt [background_color="lightblue"] "valid user" {
    client -> server: "Access granted";
} else [background_color="lightpink"] "invalid user" {
    client -> server: "Access denied";
};
```

**Note:** The `operation_label_text` attribute controls the styling of the operation label (like "alt", "loop", "opt"), while `section_title_text` controls the styling of individual section titles. This allows for independent customization of these two text elements.

#### 6.4.3 Semantics

- Sequence diagrams only: Using fragments in component diagrams is invalid
- Grouping and alternatives: Multiple sections represent distinct phases or alternative paths
- No namespace creation: Fragments do not create component namespaces; identifiers remain flat
- Nested fragments: Fragments may be nested within sections
- Ordering and elaboration: Sections’ contents are integrated into the surrounding sequence flow; later compilation phases operate on flattened elements

Scoping behavior:
- ✅ Correct: user -> server (flat naming within sections and across fragments)
- ❌ Incorrect: user::server (no namespace scoping via fragments)

Diagram type restriction:
- Fragments are only supported in sequence diagrams. Using them in component diagrams produces an error.

#### 6.4.4 Sugar Syntax

To improve ergonomics and readability, Filament provides dedicated keywords for common UML 2.5 interaction operators. These keywords are syntactic sugar that desugar to the base `fragment` syntax during compilation.

**Available Keywords:**
- `alt`/`else` - Alternatives (conditional branching)
- `opt` - Optional (single conditional path)
- `loop` - Iteration (repeated execution)
- `par` - Parallel (concurrent execution)
- `break` - Interruption (breaking from enclosing fragment)
- `critical` - Critical region (atomic execution)

**Benefits of Sugar Syntax:**
- More concise and readable code
- Clear intent through dedicated keywords
- Reduced boilerplate for common patterns
- Consistent with UML terminology

**General Pattern:**
```
keyword @TypeName [attributes...] "title" {
    // elements
};
```

**Syntactic Sugar:**

When `@TypeName` is omitted, it defaults to `@Fragment`:

The generic `fragment` syntax remains fully supported for custom operations or when explicit control is needed.

##### 6.4.3.1 alt/else - Alternatives

Represents conditional branching with mutually exclusive paths.

**Syntax:**
```
alt @TypeName [attributes...] "condition1" {
    // first alternative
} else "condition2" {
    // second alternative
} else "condition3" {
    // third alternative
};
```

**Requirements:**
- At least one `alt` block required
- Zero or more `else` clauses
- Title strings are optional for each branch
- Attributes apply to entire fragment (optional, placed after `alt`)
- Each block ends with `}`, final clause ends with `};`

**Desugars to:**
```
fragment @TypeName [attributes...] "alt" {
    section "condition1" { /* ... */ };
    section "condition2" { /* ... */ };
    section "condition3" { /* ... */ };
};
```

##### 6.4.3.2 opt - Optional

Represents a single conditional execution path.

**Syntax:**
```
opt @TypeName [attributes...] "condition" {
    // optional elements
};
```

**Requirements:**
- Single block only
- Title string is optional
- Attributes apply to the fragment (optional)

**Desugars to:**
```
fragment @TypeName [attributes...] "opt" {
    section "condition" { /* ... */ };
};
```

##### 6.4.3.3 loop - Iteration

Represents repeated execution with a loop guard condition.

**Syntax:**
```
loop @TypeName [attributes...] "guard_condition" {
    // repeated elements
};
```

**Requirements:**
- Single block only
- Title string is optional but recommended (describes loop condition)
- Attributes apply to the fragment (optional)
- Common titles: "for each", "while x > 0", "until complete"

**Desugars to:**
```
fragment @TypeName [attributes...] "loop" {
    section "guard_condition" { /* ... */ };
};
```

##### 6.4.3.4 par - Parallel

Represents concurrent execution of multiple paths.

**Syntax:**
```
par @TypeName [attributes...] "label1" {
    // first parallel path
} par "label2" {
    // second parallel path
};
```

**Requirements:**
- At least one `par` block required
- Multiple `par` blocks represent concurrent paths
- Title strings are optional for each path
- Attributes apply to entire fragment (optional, placed after first `par`)
- Each block ends with `}`, final block ends with `};`

**Desugars to:**
```
fragment @TypeName [attributes...] "par" {
    section "label1" { /* ... */ };
    section "label2" { /* ... */ };
};
```

##### 6.4.3.5 break - Interruption

Represents breaking out of an enclosing fragment, typically used for exceptions or interrupts.

**Syntax:**
```
break @TypeName [attributes...] "condition" {
    // interruption handling
};
```

**Requirements:**
- Single block only
- Title string is optional but recommended
- Attributes apply to the fragment (optional)
- Semantically breaks out of enclosing loop/alt/opt fragment

**Desugars to:**
```
fragment @TypeName [attributes...] "break" {
    section "condition" { /* ... */ };
};
```

##### 6.4.3.6 critical - Critical Region

Represents an atomic execution region that must not be interleaved.

**Syntax:**
```
critical @TypeName [attributes...] "label" {
    // atomic operations
};
```

**Requirements:**
- Single block only
- Title string is optional
- Attributes apply to the fragment (optional)
- Indicates mutual exclusion semantics

**Desugars to:**
```
fragment @TypeName [attributes...] "critical" {
    section "label" { /* ... */ };
};
```

#### 6.4.5 Examples

##### Base Fragment Examples

Basic fragment with a single section:
```
diagram sequence;

a: Rectangle;
b: Rectangle;

fragment "Minimal" {
    section {
        a -> b;
    };
};
```

Multiple sections (alternatives) and nesting:
```
diagram sequence;

user: Rectangle;
auth: Rectangle;

fragment [border_style="dashed", background_color="#f8f8f8"] "Authentication Flow" {
    section "successful login" {
        user -> auth: "Credentials";
        activate auth {
            auth -> user: "Access granted";
        };
    };
    section "failed login" {
        user -> auth: "Credentials";
        auth -> user: "Access denied";
    };
    section "nested decision" {
        fragment "Recovery" {
            section "password reset" {
                user -> auth: "Reset";
            };
            section "support" {
                user -> auth: "Open ticket";
            };
        };
    };
};
```

##### Sugar Syntax Examples

Using all fragment keywords together:

```
diagram sequence;

client: Rectangle;
server: Rectangle;
database: Rectangle;
cache: Rectangle;

// Authentication with alternatives
alt "valid credentials" {
    client -> server: "Login";
    activate server {
        server -> database: "Verify";
        database -> server: "OK";
        server -> client: "Success";
    };
} else "invalid credentials" {
    client -> server: "Login";
    server -> client: "Access denied";
};

// Optional caching
opt "cache enabled" {
    server -> cache: "Store session";
};

// Data processing loop
loop "for each item in batch" {
    client -> server: "Process item";

    critical "database transaction" {
        server -> database: "BEGIN";
        server -> database: "INSERT";
        server -> database: "COMMIT";
    };

    break "error occurred" {
        server -> client: "Error notification";
    };
};

// Parallel operations
par "fetch user data" {
    server -> database: "SELECT users";
} par "fetch settings" {
    server -> database: "SELECT settings";
};
```

Individual keyword examples:

```
// alt/else - alternatives
alt "valid token" {
    client -> server: "Proceed";
} else {
    client -> server: "Reject";
};

// opt - optional
opt "cache hit" {
    server -> cache: "Load";
};

// loop - iteration
loop "for each record" {
    client -> database: "Process";
};

// par - parallel
par "thread 1" {
    server -> db1: "Query";
} par "thread 2" {
    server -> db2: "Query";
};

// break - interruption
break "timeout" {
    client -> server: "Cancel";
};

// critical - critical region
critical "transaction" {
    server -> database: "COMMIT";
};
```

## 7. Attributes

Attributes customize the appearance and behavior of elements:

### 7.1 Attribute Value Types

Filament supports two types of attribute values: string literals and float literals. For detailed documentation on syntax, formats, and usage rules, see:

**[Literal Values and Data Types Specification](literal_values.md)**

### 7.2 Shape-specific Attributes

- `fill_color`: The background color of a shape (string, e.g., `"#ff0000"`, `"red"`, `"rgb(255,0,0)"`)
- `rounded`: Rounding radius for rectangle corners (float, e.g., `10.0`, `5.5`)
- `background_color`: When used in a diagram declaration, sets the background color of the entire diagram (string)
- `stroke`: Border/outline styling for shapes (see section 7.3 for details)

### 7.3 Stroke Attributes

Stroke attributes control the appearance of borders, lines, and outlines. They must be grouped under the `stroke` attribute using nested attribute syntax.

```
stroke=[attribute1=value1, attribute2=value2, ...]
```

Available stroke attributes within the `stroke` group:

- `color`: The stroke color (string, e.g., `"red"`, `"#ff0000"`, `"rgb(255,0,0)"`)
- `width`: The thickness of the stroke (float, e.g., `2.0`, `1.5`)
- `style`: The stroke style (string: `"solid"`, `"dashed"`, `"dotted"`, or a custom pattern like `"5,3"`)
- `line_cap`: The line cap style (string: `"butt"`, `"round"`, `"square"`)
- `line_join`: The line join style (string: `"miter"`, `"round"`, `"bevel"`)

**Custom Dash Patterns:**

The `style` attribute supports custom dash patterns specified as comma-separated numbers representing dash and gap lengths:
- `"5,3"` - 5 units dash, 3 units gap
- `"10,5,2,5"` - 10 units dash, 5 units gap, 2 units dash, 5 units gap (repeating)

Example usage for shapes:
```
type StyledBox = Rectangle [
    fill_color="lightblue",
    stroke=[color="navy", width=2.5, style="solid"]
];

type DashedBox = Rectangle [
    fill_color="white",
    stroke=[color="red", width=1.5, style="dashed"]
];

type CustomDashBox = Rectangle [
    fill_color="yellow",
    stroke=[color="black", width=2, style="10,5,2,5", line_cap="round"]
];
```

**Stroke Usage in Different Contexts:**

- **Shapes**: Use `stroke=[...]` for border styling
- **Arrows/Relations**: Use `stroke=[...]` for line styling
- **Fragments**: Use `border_stroke=[...]` for border and `separator_stroke=[...]` for internal lines
- **Lifelines**: Configured globally in diagram declaration or configuration file
- **Activation Boxes**: Configured globally in diagram declaration or configuration file

### 7.4 Text Attributes

Text attributes must be grouped under the `text` attribute using nested attribute syntax.

```
text=[attribute1=value1, attribute2=value2, ...]
```

Available text attributes within the `text` group:

- `font_size`: Size of text labels (float, e.g., `16`, `12.5`)
- `font_family`: Font family name (string, e.g., `"Arial"`, `"Courier New"`, `"Helvetica"`)
- `color`: Text color (string, e.g., `"red"`, `"#ff0000"`, `"rgb(255,0,0)"`, `"rgba(255,0,0,0.5)"`)
- `background_color`: Background color behind text (string, e.g., `"white"`, `"#f0f0f0"`, `"rgba(255,255,255,0.8)"`)
- `padding`: Padding around text content (float, e.g., `5.0`, `8.5`)

Example usage:
```
type StyledButton = Rectangle [
    fill_color="blue",
    text=[font_size=16, font_family="Arial", color="white", background_color="blue", padding=8.0]
];

type WarningButton = Rectangle [
    fill_color="white",
    text=[color="red", font_size=18, background_color="yellow", padding=5.0]
];

// Text with alpha transparency
type SemiTransparentText = Rectangle [
    fill_color="white",
    text=[color="rgba(255, 0, 0, 0.5)", font_size=16]
];
```

### 7.5 Relation-specific Attributes

- `style`: The routing style of the arrow line (string: `"straight"`, `"curved"`, or `"orthogonal"`, default is `"straight"`)
- `stroke`: Line styling for relations (see section 7.3 for details)

Example usage for relations:
```
// Basic relation with stroke styling
source -> @Arrow[stroke=[color="red", width=2.5]] target;

// Relation with dashed stroke
source -> @Arrow[stroke=[style="dashed", width=1.5], style="curved"] target;

// Relation with custom dash pattern
source -> @Arrow[stroke=[style="5,3", color="blue"]] target;
```

Relations also support all text attributes listed in section 7.4 for styling their labels, including text color.

### 7.6 Relation Labels

Relations can optionally include text labels to describe their purpose or meaning:

```
<source> <relation_type> [attributes...] <target>: "Label text";
```

Labels are displayed above the relation line with a background for readability.

## 8. Nesting and Hierarchy

Components can contain other elements, creating a hierarchical structure:

```
parent_system: Rectangle {
    child_service1: Oval;
    child_service2: Rectangle;
    child_service1 -> child_service2;
};
```

Nested components are positioned within their parent container and maintain their relationships.

### 8.1 Embedded Diagrams

Filament supports embedding different diagram types within components, allowing for richer multi-level visualizations. For example, you can embed a sequence diagram inside a component diagram to show the dynamic behavior of a component:

```
user_service: Rectangle embed diagram sequence {
    client: Rectangle;
    server: Rectangle;
    database: Rectangle;

    client -> server: "Request";
    server -> database: "Query";
    database -> server: "Results";
    server -> client: "Response";
};
```

Embedded diagrams use the following syntax:

```
<element_name> [as "Display Label"]: <type> [element_attributes...] embed diagram <diagram_kind> [diagram_attributes...] {
    // Full diagram definition for the embedded diagram
    // Elements and relations following the standard syntax for the specified diagram_kind
};
```

When a component contains an embedded diagram:
- The embedded diagram is rendered as part of the parent component
- The embedded diagram follows the syntax and layout rules of its declared type
- The parent component is sized appropriately to contain the embedded diagram
- The embedded diagram can have its own attributes like `background_color` and `layout_engine`

## 9. Identifiers and Naming Conventions

- Type identifiers must use CamelCase (e.g., `Database`, `UserService`)
- Element identifiers typically use snake_case (e.g., `auth_service`, `user_db`)
- Identifiers can include alphanumeric characters and underscores
- Nested identifiers use `::` for qualification (e.g., `parent_system::child_service1`)
- Identifiers must start with a letter
- Identifiers are case-sensitive

## 10. Layout Behavior

Filament supports multiple layout engines that can be specified using the `layout_engine` attribute in the diagram declaration:

```
diagram component [layout_engine="force", background_color="#f5f5f5"];
```

Available layout engines:

- `basic`: The default layout engine with simple positioning (available for both component and sequence diagrams)
- `force`: A force-directed layout engine for more organic component positioning (available for component diagrams)
- `sugiyama`: A hierarchical layout engine for layered diagrams (available for component diagrams)

### 10.1 Component Diagrams

- Components are automatically positioned based on their relationships
- Nested components are arranged within their parent container
- Sizes are automatically calculated based on content and text
- Margins and padding are automatically applied for readability
- The layout algorithm can be selected with the `layout_engine` attribute

### 10.2 Sequence Diagrams

- Participants (components) are arranged horizontally
- Messages (relations) are displayed as horizontal arrows between participants
- Time flows downward, with messages ordered as they appear in the source
- Lifelines extend from each participant throughout the diagram

## 11. Rendering Output

Filament diagrams are rendered as SVG files with the following characteristics:

- Components are rendered using their defined shape type
- Relations are rendered as lines with appropriate arrowheads
- Text labels are positioned appropriately for each shape type
- Nested elements are visually contained within their parents
- Component boundaries adjust to fit their content
- Boundary shapes render as fixed-size UML boundary symbols with text labels positioned below

### 11.1 Content-Free Shapes

Some shapes, like `Boundary`, `Actor`, `Entity`, `Control`, and `Interface`, are content-free and cannot contain nested elements or embedded diagrams. These shapes are designed for specific purposes such as representing external actors, entities, control elements, interfaces, or system boundaries in UML diagrams.

Content-free shapes have the following characteristics:
- They cannot contain nested components
- They cannot have embedded diagrams
- Their text labels appear below the shape rather than within it
- They have a fixed size that is not affected by content

Attempting to add nested content to a content-free shape will result in an error:

```
// These will cause errors:
user_actor: Boundary {
    internal_service: Rectangle; // Error: Boundary shapes cannot contain content
};
```

## 12. Complete Examples

### 12.1 Component Diagram Example

```
diagram component [layout_engine="force", background_color="#f8f8f8"];

// Define component types
type Database = Rectangle [fill_color="lightblue", rounded=10];
type Service = Component [fill_color="#e6f3ff"];
type Client = Oval [fill_color="#ffe6e6"];

// Define relation types
type RedArrow = Arrow [stroke=[color="red"]];
type BlueArrow = Arrow [stroke=[color="blue", width=2.0]];

// Define relation types extending other custom types
type ThickRedArrow = RedArrow [stroke=[width=3.0], text=[font_size=16]];
type OrthogonalBlueArrow = BlueArrow [style="orthogonal"];

end_user as "End User": Client;
backend_system as "Backend System": Service {
    auth_service as "Auth Service": Service;
    user_db: Database;
    auth_service -> user_db;
};
api_gateway: Service;

end_user -> api_gateway;
api_gateway -> @ThickRedArrow backend_system;
api_gateway -> @RedArrow[ style="curved"] end_user: "Response";
backend_system -> @OrthogonalBlueArrow user_database: "Query";
end_user -> @BlueArrow auth_service: "Auth requests";
```

### 12.2 Sequence Diagram Example

Basic sequence diagram with fragment keyword sugar syntax:

```
diagram sequence;

client: Rectangle;
server: Rectangle;
database: Rectangle;
cache: Rectangle;

// Authentication flow with alternatives
alt "valid token" {
    client -> server: "Request with token";
    activate server {
        server -> database: "Verify token";
        database -> server: "Valid";
        server -> client: "Success";
    };
} else "invalid token" {
    client -> server: "Request";
    server -> client: "401 Unauthorized";
};

// Optional caching
opt "cache hit" {
    server -> cache: "Check cache";
    cache -> server: "Cached data";
};

// Data processing loop
loop "for each page" {
    client -> server: "Fetch page";
    server -> database: "Query";
    database -> server: "Results";
    server -> client: "Page data";
};

// Parallel database operations
par "update user table" {
    server -> database: "UPDATE users";
} par "update activity log" {
    server -> database: "INSERT INTO logs";
};

// Critical section for transaction
critical "payment transaction" {
    server -> database: "BEGIN TRANSACTION";
    server -> database: "UPDATE balance";
    server -> database: "INSERT payment_log";
    server -> database: "COMMIT";
};
```

The same diagram using base `fragment` syntax:

```
diagram sequence;

client: Rectangle;
server: Rectangle;
database: Rectangle;
cache: Rectangle;

fragment "alt" {
    section "valid token" {
        client -> server: "Request with token";
        activate server {
            server -> database: "Verify token";
            database -> server: "Valid";
            server -> client: "Success";
        };
    };
    section "invalid token" {
        client -> server: "Request";
        server -> client: "401 Unauthorized";
    };
};

fragment "opt" {
    section "cache hit" {
        server -> cache: "Check cache";
        cache -> server: "Cached data";
    };
};

fragment "loop" {
    section "for each page" {
        client -> server: "Fetch page";
        server -> database: "Query";
        database -> server: "Results";
        server -> client: "Page data";
    };
};

fragment "par" {
    section "update user table" {
        server -> database: "UPDATE users";
    };
    section "update activity log" {
        server -> database: "INSERT INTO logs";
    };
};

fragment "critical" {
    section "payment transaction" {
        server -> database: "BEGIN TRANSACTION";
        server -> database: "UPDATE balance";
        server -> database: "INSERT payment_log";
        server -> database: "COMMIT";
    };
};
```

### 12.3 Embedded Diagram Example

```
diagram component [background_color="#f8f8f8"];

type Service = Rectangle [fill_color="#e6f3ff"];
type Database = Rectangle [fill_color="lightblue", rounded=10];
type SecureArrow = Arrow [stroke=[color="orange", width=2.0]];

user_interface: Oval [fill_color="#ffe6e6"];
auth_service: Service embed diagram sequence {
    client: Rectangle;
    auth: Rectangle;
    database: Rectangle;

    client -> auth: "Login Request";
    auth -> database: "Validate";
    database -> auth: "Result";
    auth -> client: "Auth Token";
};
database: Database;

user_interface -> @SecureArrow auth_service: "Secure connection";
auth_service -> database;
```

## 13. Error Handling

Filament provides comprehensive error handling with precise location tracking and user-friendly error messages. For detailed information about error handling architecture, message formats, and implementation details, see:

**[Error Handling Specification](error_handling.md)**

## 14. Configuration File

Filament supports configuration through a TOML file that can specify default settings for diagram rendering.

### 14.1 Configuration File Locations

Filament searches for configuration files in the following locations (in order of priority):

1. Explicitly provided path with the `-c/--config` command-line option
2. Local directory: `./filament/config.toml`
3. Platform-specific user config directory: `config.toml` in the standard configuration directory for your platform

   The specific paths follow the [directories](https://docs.rs/directories/latest/directories/) crate's `ProjectDirs` convention, using the qualifier "com", organization "filament", and application name "filament".

If no configuration file is found, default values are used.

### 14.2 Configuration File Format

The configuration file uses TOML syntax and supports the following settings:

```toml
# Layout engine configuration
[layout]
# Default layout engine for component diagrams (basic, force, sugiyama)
component = "sugiyama"
# Default layout engine for sequence diagrams (basic)
sequence = "basic"

# Style configuration
[style]
# Default background color for diagrams
background_color = "#f5f5f5"

# Lifeline stroke configuration for sequence diagrams
[lifeline]
color = "black"
width = 1.0
style = "dashed"
line_cap = "butt"
line_join = "miter"

```

Layout engine values are case-sensitive and must match the supported enum values exactly.
Color values must be valid CSS color strings.

### 14.3 Layout Engine Values

The layout engine names in the configuration file are string representations of the internal enum values:

| String Value | Layout Engine Type | Supported Diagram Types       |
|--------------|-------------------|------------------------------|
| "basic"      | Basic layout      | Component, Sequence          |
| "force"      | Force-directed    | Component                    |
| "sugiyama"   | Hierarchical      | Component                    |

### 14.4 Style Configuration

The style configuration section controls the visual appearance of diagrams:

- `background_color`: Sets the default background color for all diagrams
  - Accepts any valid CSS color string (e.g., `"#f5f5f5"`, `"white"`, `"rgb(240,240,240)"`)
  - Can be overridden by the `background_color` attribute in individual diagram declarations

### 14.5 Sequence Diagram Stroke Configuration

**Lifeline Stroke Configuration:**

The `[lifeline]` section configures the appearance of lifelines in sequence diagrams:

- `color`: Lifeline stroke color (string, e.g., `"black"`, `"#000000"`)
- `width`: Lifeline stroke width (float, e.g., `1.0`, `1.5`)
- `style`: Lifeline stroke style (string: `"solid"`, `"dashed"`, `"dotted"`, or custom pattern like `"5,3"`)
- `line_cap`: Line cap style (string: `"butt"`, `"round"`, `"square"`)
- `line_join`: Line join style (string: `"miter"`, `"round"`, `"bevel"`)



### 14.6 Configuration Priority

When determining which styles or layout engines to use, Filament follows this priority order:

#### Layout Engine Priority

1. Explicit layout engine in diagram declaration (`layout_engine` attribute)
2. Default layout engine in configuration file (if found in any of the search locations)
3. Built-in default (`basic`)

#### Style Priority

For styling attributes like background color:

1. Explicit attribute in diagram declaration (e.g., `background_color` attribute)
2. Default value in configuration file (if found in any of the search locations)
3. Built-in default (transparent)

#### Embedded Diagram Priority

For embedded diagrams:

1. Attributes specified in the embedded diagram declaration take precedence over inherited attributes
2. If not specified, embedded diagrams inherit layout engine settings from the configuration file
3. If neither is available, embedded diagrams use their type-specific built-in defaults

## 15. Command Line Usage

Filament diagrams can be rendered using the command line tool:

```
filament [--log-level=LEVEL] [-c|--config=CONFIG.toml] [-o|--output=FILE.svg] input_file.fil
```

Where:
- `--log-level`: Sets the logging verbosity (off, error, warn, info, debug, trace)
- `-c, --config`: Path to a TOML configuration file (optional)
- `-o, --output`: Specifies the output SVG file path (defaults to "out.svg")
- `input_file.fil`: The path to the Filament source file
