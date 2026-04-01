# Orrery Import System Specification

## 1. Introduction

Orrery's import system enables reuse and composition across `.orr` files. It serves two primary goals:

1. **Type Reuse** — Bring pre-defined types into scope so they can be shared across multiple files without duplication.
2. **Diagram Embedding** — Reference existing diagram files and embed them within components that support content.

The import system builds on the existing [Type System](type_system.md) and integrates with the [Embedded Diagrams](specification.md#81-embedded-diagrams) feature.

## 2. File Types

Orrery distinguishes two kinds of `.orr` files based on their header declaration.

### 2.1 Diagram Files

Diagram files are full, renderable diagrams. They begin with a `diagram` header:

```
diagram sequence;
```

Diagram files can contain imports, type definitions, and diagram elements (components, relations, constructs).

### 2.2 Library Files

Library files contain only reusable definitions. They begin with a `library` header:

```
library;
```

Library files can contain `import` declarations and `type` definitions. They **cannot** contain diagram elements and are **not renderable**.

### 2.3 Comparison

| Feature | Diagram File | Library File |
|---|---|---|
| Header | `diagram <kind> [attributes...];` | `library;` |
| Contains imports | Yes | Yes |
| Contains type definitions | Yes | Yes |
| Contains diagram elements | Yes | No |
| Renderable | Yes | No |
| Can be embedded via import | Yes | No |

## 3. Import Syntax

The `import` keyword brings types and diagram references from other `.orr` files into the current file's scope.

### 3.1 Three Import Forms

Orrery supports three distinct import forms:

#### 3.1.1 Namespaced Import

Imports all types behind a namespace. Types are accessed via `namespace::TypeName`.

**Syntax:**
```
import "path";
```

**Example:**
```
import "shared/styles";

api: styles::Service;
db: styles::Database;
```

#### 3.1.2 Glob Import

Imports all types flat into the current scope. No namespace prefix is required.

**Syntax:**
```
import "path"::*;
```

**Example:**
```
import "shared/styles"::*;

api: Service;
db: Database;
```

#### 3.1.3 Selective Import

Imports only the named types flat into the current scope. Individual types can be aliased using the `as` keyword.

**Syntax:**
```
import "path"::{TypeA, TypeB};
import "path"::{TypeA as Alias, TypeB};
```

**Example:**
```
import "shared/styles"::{Service, Database};
import "shared/styles"::{Service as Svc, Database as DB};

api: Service;
db: Database;
```

### 3.2 Summary

| Form | Syntax | Result |
|---|---|---|
| Namespaced | `import "path";` | All types behind namespace, accessed as `name::Type` |
| Glob | `import "path"::*;` | All types flat in current scope, no namespace |
| Selective | `import "path"::{A, B};` | Named types flat in current scope |

## 4. Import Paths

### 4.1 String Literal Format

Import paths are expressed as string literals:

```
import "shared/styles";
import "../common/base";
```

### 4.2 No File Extension

The `.orr` extension is **omitted** from import paths. The compiler appends `.orr` when resolving the path.

```
import "shared/styles";       // resolves to shared/styles.orr
import "../common/base";      // resolves to ../common/base.orr
```

### 4.3 Relative Resolution

Import paths are resolved **relative to the importing file's directory**. There are no global search paths, bare module names, or project-root-relative resolution.

### 4.4 Path Rules

- Paths use **forward slashes only** (`/`), regardless of operating system.
- The `.orr` extension is **never** included in the path.
- Circular import dependencies are **not** permitted and are a compile-time error.

## 5. Namespaces

### 5.1 Derived Namespace Name

For namespaced imports, the namespace name is derived from the **last segment** of the import path:

```
import "shared/styles";       // namespace: styles
import "../common/base";      // namespace: base
import "auth_flow";           // namespace: auth_flow
```

### 5.2 Namespace Override with `as`

The derived namespace name can be overridden using the `as` keyword:

```
import "shared/styles" as theme;   // namespace: theme (not styles)
import "../common/base" as core;   // namespace: core (not base)
```

### 5.3 Namespace Access with `::`

Types within a namespace are accessed using the `::` operator:

```
import "shared/styles";

api: styles::Service;
db: styles::Database;
arrow: styles::DashedArrow;
```

## 6. Diagram Embedding via Import

### 6.1 Namespaced Import Embedding

A namespaced import of a diagram file creates an **embed reference**. The namespace identifier doubles as the embed name, which can be used with the `embed` keyword in component declarations:

```
import "auth_flow";

auth_box: Rectangle embed auth_flow;
```

This is equivalent to embedding the full diagram inline but references an external file instead.

### 6.2 Namespace Override for Embed Names

The `as` keyword can customize the embed reference name:

```
import "diagrams/complex_authentication_flow" as auth;

auth_box: Rectangle embed auth;
```

### 6.3 Restrictions

- **Only namespaced imports** create embed references. Glob and selective imports do **not** create embed references.
- **Only diagram files** can be embedded. Attempting to embed a library file is a compile-time error.

| Import Form | Creates Embed Reference |
|---|---|
| `import "diagram_file";` | Yes |
| `import "diagram_file" as name;` | Yes (uses alias) |
| `import "diagram_file"::*;` | No |
| `import "diagram_file"::{A, B};` | No |

### 6.4 Comparison with Inline Embedding

Orrery supports two approaches to embedding diagrams:

**Inline embedding** (defined in [Embedded Diagrams](specification.md#81-embedded-diagrams)):
```
user_service: Rectangle embed {
    diagram sequence;

    client: Rectangle;
    server: Rectangle;
    client -> server: "Request";
};
```

**Import-based embedding**:
```
import "user_service_flow";

user_service: Rectangle embed user_service_flow;
```

## 7. Scope and Visibility

### 7.1 Everything Is Public

Orrery has **no visibility modifiers**. All types and definitions in a **library** file are accessible to any file that imports it.

Diagram files are self-contained renderable units. Their type definitions are **internal** to the diagram and are **not** exported to importers. Importing a diagram file creates an embed reference only (see [§6](#6-diagram-embedding-via-import)).

### 7.2 Transitive Re-Export

Transitive re-export is the **default** behavior for **library** files. A library file's namespace exposes everything visible in that file's scope — its own type definitions plus everything it imported.

Diagram file types are **not** re-exported. Importing a diagram file provides only an embed reference; the diagram's internal types remain invisible to the importer.

**Example:**

`base.orr`:
```
library;

type Service = Rectangle[fill_color="lightblue"];
type Database = Oval[fill_color="lightgreen"];
```

`extended.orr`:
```
library;

import "base"::*;

type SecureService = Service[stroke=[color="red"]];
```

`main.orr`:
```
diagram component;

import "extended";

// Own types from extended
api: extended::SecureService;

// Transitively re-exported from base via extended
db: extended::Database;
svc: extended::Service;
```

### 7.3 Chained Access

Transitive visibility enables chained namespace access:

```
import "parent";

item: parent::child::TypeName;
```

This works when `parent.orr` contains `import "child";` (namespaced), making `child`'s types accessible through `parent`'s namespace.

## 8. Conflict Resolution

### 8.1 Last Writer Wins

When multiple imports or definitions introduce the same type name into the flat scope, the **last definition wins**. The order of import statements and type definitions matters.

```
import "theme_a"::*;   // defines Service with fill_color="blue"
import "theme_b"::*;   // defines Service with fill_color="red"

api: Service;           // uses theme_b's Service (last writer wins)
```

### 8.2 Conflicts Are Not Errors

Name conflicts in the flat scope are **not** compile-time errors.

### 8.3 Local Definitions Override Imports

Local type definitions override any imported type with the same name:

```
import "shared/styles"::*;   // defines Service

type Service = Rectangle[fill_color="custom"];  // overrides imported Service

api: Service;  // uses local Service
```

### 8.4 Namespaced Imports Avoid Conflicts

Namespaced imports avoid flat-scope conflicts entirely, since types are accessed via their namespace prefix:

```
import "theme_a";
import "theme_b";

blue_api: theme_a::Service;
red_api: theme_b::Service;
```

## 9. File Structure

### 9.1 Declaration Order

An Orrery file follows a strict ordering of declarations:

1. **File header** — `diagram <kind> [attributes...];` or `library;`
2. **Import declarations** — All `import` statements
3. **Type definitions** — All `type` definitions
4. **Diagram elements** — Components, relations, constructs (diagram files only)

```
diagram sequence;              // 1. header

import "shared/styles";        // 2. imports
import "common/types"::*;

type Custom = Service[...];    // 3. type definitions
type Special = Custom[...];

client: Custom;                // 4. elements
server: Special;
client -> server: "Request";
```

### 9.2 Library File Structure

Library files follow the same ordering but omit diagram elements:

```
library;                       // 1. header

import "base/types"::*;       // 2. imports

type Service = Rectangle[...]; // 3. type definitions
type Database = Oval[...];
```

## 10. Complete Examples

### 10.1 Shared Type Library

`shared/styles.orr`:
```
library;

type DashedLine = Stroke[style="dashed", color="grey"];
type Service = Rectangle[fill_color="lightblue", stroke=DashedLine];
type Database = Oval[fill_color="lightgreen"];
type DashedArrow = Arrow[stroke=[style="dashed"]];
```

### 10.2 Extended Library with Imports

`shared/secure.orr`:
```
library;

import "styles"::*;

type SecureService = Service[stroke=[color="red", width=2.0]];
type CriticalService = SecureService[fill_color="darkred", text=[color="white"]];
```

### 10.3 Reusable Diagram

`diagrams/auth_flow.orr`:
```
diagram sequence;

import "../shared/styles"::*;

client: Service;
server: Service;
database: Database;

client -> server: "Login Request";
server -> database: "Verify Credentials";
database -> server: "Auth Token";
server -> client: "Login Response";
```

### 10.4 Main Diagram with All Import Forms

`diagrams/main.orr`:
```
diagram component;

// Namespaced import — types accessed via namespace, diagram available for embedding
import "auth_flow";

// Glob import — all types flat in scope
import "../shared/styles"::*;

// Selective import with alias
import "../shared/secure"::{SecureService, CriticalService as Critical};

// Local type using imported base
type Gateway = Service[rounded=10, fill_color="orange"];

// Elements using various imported types
api_gateway: Gateway;
auth_service: SecureService;
core_service: Critical;
user_db: Database;

// Import-based diagram embedding
auth_detail: Rectangle embed auth_flow;

// Relations
api_gateway -> auth_service: "Authenticate";
api_gateway -> core_service: "Process";
auth_service -> @DashedArrow user_db: "Query";
core_service -> user_db: "Read";
```

### 10.5 File Structure Summary

```
project/
├── shared/
│   ├── styles.orr         # library — base types
│   └── secure.orr         # library — imports styles, adds secure types
└── diagrams/
    ├── auth_flow.orr      # diagram — reusable sequence diagram
    └── main.orr           # diagram — imports everything, embeds auth_flow
```
