# Orrery Type System Specification

## 1. Introduction

The Orrery Type System provides a unified, generic, and compositional framework for defining and applying types (styles) to all language constructs.

## 2. Core Principles

The Type System is built on one fundamental distinction:

### 2.1 Declarations vs. Invocations

The language distinguishes between two distinct operations:

#### 2.1.1 Declarations (`:`)

Allocating a named instance that can be referenced later.

**Syntax:** `identifier: Type`

**Example:**
```
api_gateway: Service;
database: Rectangle;
```

#### 2.1.2 Invocations (`@`)

Executing an anonymous action (drawing an arrow, note, activation, or fragment).

**Syntax:** `keyword @Type`

**Example:**
```
note @WarningNote: "This is important";
client -> @DashedArrow server: "Request";
```

## 3. The `type` Declaration

The `type` keyword binds a new name to a **Type Specifier**. This allows creating new types by extending either built-in base types or other previously defined user types.

### 3.1 Syntax

```
type <NewTypeName> = <TypeSpec>;
```

Where:
- `<NewTypeName>` is a CamelCase identifier starting with an uppercase letter
- `<TypeSpec>` is a Type Specifier (see Section 4)

### 3.2 Examples

```
// Extending a built-in base type
type DashedLine = Stroke[style="dashed", color="grey"];
type Service = Rectangle[fill_color="lightblue", stroke=DashedLine];

// Extending a user-defined type
type ThickDashedLine = DashedLine[width=2.0];
type AdminService = Service[fill_color="darkblue", text=[color="white"]];
```

### 3.3 Type Composition

Types can be composed by extending other types and overriding or adding attributes. The extension syntax `[...]` is compositional and works at any level:

```
type BaseService = Rectangle[fill_color="lightblue"];
type SecureService = BaseService[stroke=[color="red", width=2.0]];
type CriticalService = SecureService[rounded=10, fill_color="darkred"];
```

Each extension builds upon the previous type, with later attributes overriding earlier ones.

## 4. Type Specifier

A **Type Specifier** (TypeSpec) is the core syntax for defining and using types. It appears in both `type` declarations (Section 3) and at the point of use (Sections 6-7).

### 4.1 Two Forms of TypeSpec

A TypeSpec can be expressed in one of two forms:

#### 4.1.1 Named

A reference to a previously defined type (via `type` declaration) or a built-in base type.

**Syntax:** `TypeName`

**Examples:**
```
Service             // user-defined type
WarningNote         // user-defined type
Rectangle           // built-in base type
Arrow               // built-in base type
```

#### 4.1.2 Anonymous

A type created on-the-spot with attributes, without assigning it a name. Can extend either a base type or a named type.

**Syntax:** `TypeName[attribute1=value1, attribute2=value2, ...]`

**Examples:**
```
// Extending base types
Rectangle[fill_color="red"]
Stroke[width=2.0, style="dashed"]
Arrow[color="blue"]
Note[background_color="yellow"]

// Extending named types
Service[rounded=0, fill_color="green"]
WarningNote[text=[color="black"]]
DashedLine[color="red"]
```

Anonymous types are used at the point of use and cannot be referenced elsewhere since they have no assigned name.

### 4.2 Usage Context

TypeSpec appears in two contexts:

1. **In `type` declarations:** Defines what the new type extends
   ```
   type MyService = Rectangle[fill_color="blue"];
   ```

2. **At point of use:** Specifies the type for a declaration or invocation
   ```
   api: Service;                               // Declaration (Named TypeSpec)
   api2: Service[rounded=5];                   // Declaration (Anonymous TypeSpec)
   note @WarningNote: "Alert";                 // Invocation (Named TypeSpec)
   client -> @Arrow[color="red"] server;       // Invocation (Anonymous TypeSpec)
   ```

## 5. Built-in Base Types

The language provides a set of built-in types that serve as the foundation for all type definitions.

### 5.1 Component Base Types

Component base types are used for defining new component shapes.

| Base Type | Description | Accepts Nested Elements |
|-----------|-------------|------------------------|
| `Rectangle` | A rectangular shape with customizable properties | Yes |
| `Oval` | An elliptical shape with customizable properties | Yes |
| `Component` | A UML-style component shape with a rectangular body and component icon | Yes |
| `Boundary` | A UML boundary shape (circle with vertical line), representing external actors or system boundaries | No |
| `Actor` | A UML actor shape (stick figure), representing external users or systems | No |
| `Entity` | A UML entity shape (circle), representing data entities or business objects | No |
| `Control` | A UML control shape (circle with arrow), representing control logic or processes | No |
| `Interface` | A UML interface shape (circle), representing system interfaces or contracts | No |

**Content-Free Shapes:**

Some base types (`Boundary`, `Actor`, `Entity`, `Control`, `Interface`) are "content-free," meaning they cannot contain nested elements. These shapes display their text labels below the shape rather than within it.

### 5.2 Relation Base Type

| Base Type | Description |
|-----------|-------------|
| `Arrow` | Base type for relations, supporting attributes like color, width, and style |

### 5.3 Construct Base Types

| Base Type | Description |
|-----------|-------------|
| `Note` | Base type for annotations and documentation notes |
| `Activate` | Base type for activation blocks in sequence diagrams |
| `Fragment` | Base type for fragment blocks (alt, opt, loop, etc.) in sequence diagrams |

### 5.4 Attribute Group Base Types

| Base Type | Description |
|-----------|-------------|
| `Stroke` | Base type for defining reusable groups of stroke attributes (color, width, style, etc.) |
| `Text` | Base type for defining reusable groups of text attributes (font, size, color, etc.) |

**Note:** Attribute group types (`Stroke`, `Text`) are used within other type definitions to group related attributes, not for direct instantiation.

### 5.5 Usage Examples

```
// Component types
type Service = Rectangle[fill_color="lightblue"];
type Database = Oval[fill_color="lightgreen"];
type ExternalAPI = Boundary;

// Relation types
type DashedArrow = Arrow[stroke=[style="dashed"]];
type ThickArrow = Arrow[stroke=[width=3.0]];

// Construct types
type WarningNote = Note[background_color="yellow"];
type CriticalActivation = Activate[fill_color="rgba(255,0,0,0.1)"];
type HighlightedFragment = Fragment[background_color="lightblue"];

// Attribute group types
type RedStroke = Stroke[color="red", width=2.0];
type BoldText = Text[font_weight="bold", font_size=14];
```

## 6. Syntactic Patterns

The TypeSpec from Section 4 is applied in two distinct patterns corresponding to the two core operations (Section 2.2).

### 6.1 Pattern 1: Declarations (Named Instances)

This pattern is used for **component definitions** where you want to create a named instance that can be referenced later.

#### 6.1.1 Syntax

**Named TypeSpec:**
```
identifier: TypeName;
```

**Anonymous TypeSpec:**
```
identifier: TypeName[attributes...];
```

Where:
- `identifier` is the name of the component (snake_case)
- `TypeName` is either a built-in base type or a user-defined type
- `[attributes...]` within TypeName makes it anonymous

#### 6.1.2 Examples

```
// Type definitions
type Service = Rectangle[fill_color="lightblue"];

// Named TypeSpec
api_gateway: Service;

// Anonymous TypeSpec (extending named type)
auth_service: Service[rounded=0];

// Anonymous TypeSpec (extending base type)
database: Rectangle[fill_color="red"];
```

#### 6.1.3 Instance Attribute Overrides

Instance attributes (in the second `[...]` block) override type attributes:

```
type BlueService = Rectangle[fill_color="blue", rounded=5];

// Instance attributes override type attributes
special: BlueService [fill_color="red"];  // Will be red, not blue
```

### 6.2 Pattern 2: Invocations (Anonymous Statements)

This pattern is used for **relations, notes, fragments, and activations**—constructs that perform actions rather than declare named entities.

#### 6.2.1 Syntax

The invocation pattern uses `@` to apply a type to an action:

**Named TypeSpec:**
```
keyword @TypeName ...
```

**Anonymous TypeSpec:**
```
keyword @TypeName [attributes...] ...
```

Where:
- `keyword` varies by construct (relation operators, `note`, `activate`, fragment keywords)
- `@TypeName` is the type being applied
- `[attributes...]` within TypeName makes it anonymous
- `...` represents construct-specific syntax (targets, labels, content, blocks)

The `@` symbol clearly indicates "perform this action with the style of this type."

## 7. Syntactic Sugar

For common cases, Orrery allows omitting the `@TypeSpec` to maintain clean and simple syntax. When omitted, the language applies a default base type for that construct. Instance attributes can still be applied to the default type.

### 7.1 Overview

| Construct | Default Type When Omitted | Syntax Pattern |
|-----------|---------------------------|----------------|
| Relations | `@Arrow` | `source -> target` |
| Notes | `@Note` | `note: "content"` |
| Activations | `@Activate` | `activate component { }` |
| Fragments | `@Fragment` | `keyword "title" { }` |

### 7.2 Relations

**Sugar Syntax:**
```
client -> server;
client -> server: "label";
client -> [color="red"] server;
client -> [color="red"] server: "label";
```

**Desugars To:**
```
client -> @Arrow server;
client -> @Arrow server: "label";
client -> @Arrow [color="red"] server;
client -> @Arrow [color="red"] server: "label";
```

### 7.3 Notes

**Sugar Syntax:**
```
note: "content";
note [align="left"]: "content";
note [background_color="yellow"]: "content";
```

**Desugars To:**
```
note @Note: "content";
note @Note [align="left"]: "content";
note @Note [background_color="yellow"]: "content";
```

### 7.4 Activations

**Sugar Syntax:**
```
activate component { };
activate [fill_color="red"] component { };
activate component;
deactivate component;
```

**Desugars To:**
```
activate @Activate component { };
activate @Activate [fill_color="red"] component { };
activate @Activate component;
deactivate component;
```

### 7.5 Fragments

**Sugar Syntax:**
```
opt "condition" { };
opt [background_color="yellow"] "condition" { };
alt "path A" { } else "path B" { };
alt [border_stroke=[width=2.0]] "path A" { } else "path B" { };
```

**Desugars To:**
```
opt @Fragment "condition" { };
opt @Fragment [background_color="yellow"] "condition" { };
alt @Fragment "path A" { } else "path B" { };
alt @Fragment [border_stroke=[width=2.0]] "path A" { } else "path B" { };
```

### 7.6 Mixing Sugar and Explicit Syntax

Sugar and explicit syntax can be mixed freely in the same diagram:

```
client: Rectangle;
server: Rectangle;

// Sugar syntax
client -> server: "Request";
note: "Processing";

// Explicit syntax
type ImportantArrow = Arrow[stroke=[color="red", width=2.0]];
client -> @ImportantArrow server: "Important";
note @Note[background_color="yellow"]: "Warning";

// Mixed
activate server {
    // Sugar inside explicit
    server -> client: "Response";
};
```
