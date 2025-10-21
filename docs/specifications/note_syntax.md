# Filament Note Syntax Specification

Notes are annotations attached to diagram elements or positioned in the diagram margins. They provide additional context, documentation, or explanatory text without participating in the diagram's structural relationships. This specification defines the syntax, attributes, and usage patterns for notes in Filament diagrams.

## 1. Overview

The note system in Filament is designed around a minimal, attribute-driven syntax that provides flexible positioning and attachment capabilities. Notes can be:

- **Attached to elements**: Positioned relative to specific diagram components
- **Spanning multiple elements**: Displayed across several components
- **Positioned in margins**: Placed in diagram margins independent of elements

## 2. Core Syntax

A note is declared using the `note` keyword followed by an attribute block and content:

```filament
note [attribute1=value, attribute2=value, ...]: "Note content";
```

**Syntax Components:**
- `note` keyword initiates the note declaration
- `[...]` attribute block defines positioning and behavior
- `:` separator between attributes and content
- `"..."` string literal containing the note text
- `;` statement terminator

## 3. Attributes

Notes are controlled through attributes. All note behavior, including positioning, alignment, and association with diagram elements, is specified in the attribute block.

### 3.1 Attribute Reference

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `on` | List of identifiers | No | Elements the note is attached to or spans |
| `align` | String | No | Note placement relative to target(s) |
| `stroke` | Stroke attributes | No | Border styling for the note box |
| `text` | Text attributes | No | Text styling for the note content |
| `background_color` | String | No | Background color of the note box |

### 3.2 The `on` Attribute

The `on` attribute specifies which diagram element(s) the note is associated with. Its value determines the note's positioning behavior.

#### 3.2.1 Value Format

The `on` attribute accepts a list of element identifiers enclosed in square brackets:

```filament
on=[element_name]              // Single element identifier
on=[element1, element2]        // Multiple element identifiers
on=[]                          // Empty list (margin note)
```

Element identifiers refer to elements defined in the diagram:

```filament
server: Rectangle;
database: Rectangle;

note [on=[server]]: "Note attached to server element";
note [on=[server, database]]: "Note spanning both elements";
```

#### 3.2.2 Behavior by Value

**Single Element (Attached Note):**
```filament
note [on=[api_gateway]]: "This note is attached to api_gateway";
```
The note is positioned relative to one specific element.

**Multiple Elements (Spanning Note):**
```filament
note [on=[client, server]]: "This note spans from client to server";
```
The note spans across the listed elements. In sequence diagrams, this is typically rendered "over" the participants. In component diagrams, the note spans across the specified components.

**Empty List or Omitted (Margin Note):**
```filament
note [on=[]]: "This note appears in the diagram margin";
note []: "Omitting 'on' also creates a margin note";
```
The note is positioned in the diagram's margin, independent of any specific element.

### 3.3 The `align` Attribute

The `align` attribute controls note placement. Valid values depend on the diagram type.

#### 3.3.1 Alignment in Sequence Diagrams

Sequence diagrams support the following alignments:

| Value | Description |
|-------|-------------|
| `"over"` | Note positioned over the element(s) (default) |
| `"left"` | Note positioned to the left (behavior depends on `on` value) |
| `"right"` | Note positioned to the right (behavior depends on `on` value) |

**Alignment Behavior Based on `on` Attribute:**

| `on` Value | `align` Value | Behavior |
|------------|---------------|----------|
| Empty or omitted | `"over"` (default) | Note appears over all elements in the diagram |
| Empty or omitted | `"left"` | Note positioned in the left margin |
| Empty or omitted | `"right"` | Note positioned in the right margin |
| Contains element(s) | `"over"` (default) | Note appears over the specified element(s) |
| Contains element(s) | `"left"` | Note positioned to the left of the specified element(s) |
| Contains element(s) | `"right"` | Note positioned to the right of the specified element(s) |

**Examples:**
```filament
// "over" alignment
note [on=[server]]: "Over the server (default)";
note [on=[server], align="over"]: "Explicitly over the server";
note [on=[client, server], align="over"]: "Spanning over client and server";
note [align="over"]: "Over all elements (default for empty on)";

// "left" and "right" with empty on (margin notes)
note [align="left"]: "Left margin note";
note [align="right"]: "Right margin note";

// "left" and "right" with specific elements
note [on=[server], align="left"]: "To the left of server";
note [on=[server], align="right"]: "To the right of server";
```

#### 3.3.2 Alignment in Component Diagrams

Component diagrams support the following alignments:

| Value | Description |
|-------|-------------|
| `"left"` | Note positioned to the left of the element(s) |
| `"right"` | Note positioned to the right of the element(s) |
| `"top"` | Note positioned above the element(s) |
| `"bottom"` | Note positioned below the element(s) (default) |

These alignments work consistently regardless of whether `on` is empty (margin), contains a single element, or contains multiple elements.

**Examples:**
```filament
note [on=[database]]: "Below the database (default)";
note [on=[database], align="left"]: "Left of the database";
note [on=[api, gateway], align="top"]: "Above api and gateway";
note [align="bottom"]: "Bottom margin note (default)";
note [align="left"]: "Left margin note";
```

### 3.4 Default Behavior

When attributes are omitted, the following defaults apply:

- **`on` not specified**: Treated as `on=[]` (empty list)
- **`align` not specified**:
  - **Sequence diagrams**: Defaults to `"over"`
  - **Component diagrams**: Defaults to `"bottom"`

**Sequence Diagram Defaults:**
```filament
diagram sequence;
// These are equivalent - note over all elements:
note: "Over all elements";
note []: "Over all elements";
note [on=[]]: "Over all elements";
note [align="over"]: "Over all elements";
note [on=[], align="over"]: "Over all elements";
```

**Component Diagram Defaults:**
```filament
diagram component;
// These are equivalent - bottom margin note:
note: "Bottom margin note";
note []: "Bottom margin note";
note [on=[]]: "Bottom margin note";
note [align="bottom"]: "Bottom margin note";
note [on=[], align="bottom"]: "Bottom margin note";
```

### 3.5 Styling Attributes

Notes support visual customization through styling attributes.

#### 3.5.1 The `stroke` Attribute

Controls the border of the note box using nested stroke attributes:

```filament
note [stroke=[color="blue", width=2.0, style="dashed"]]: "Note with custom border";
```

For complete stroke attribute documentation, see [Literal Values and Data Types Specification](literal_values.md#5-stroke-attribute-usage-and-examples).

#### 3.5.2 The `text` Attribute

Controls the text styling within the note using nested text attributes:

```filament
note [text=[font_size=14, color="darkblue", font_family="Arial"]]: "Note with custom text";
```

For complete text attribute documentation, see [Literal Values and Data Types Specification](literal_values.md#4-text-attribute-usage-and-examples).

#### 3.5.3 The `background_color` Attribute

Sets the background color of the note box:

```filament
note [background_color="lightyellow"]: "Note with yellow background";
note [background_color="#ffe4b5"]: "Note with hex color background";
note [background_color="rgba(255, 255, 200, 0.8)"]: "Note with transparent background";
```

#### 3.5.4 Combined Styling

All styling attributes can be combined:

```filament
note [
    on=[database],
    align="left",
    background_color="lightyellow",
    stroke=[color="orange", width=2.0],
    text=[font_size=12, color="darkred"]
]: "Fully styled note";
```

## 4. Content Formatting

Note content is provided as a string literal following the colon separator. The string follows Filament's standard string literal rules, supporting escape sequences:

```filament
note [on=[component]]: "Simple text content";

note [on=[component]]: "Text with \"quoted\" content";

note [on=[component]]: "Multi-line\ncontent\nsupported";

note [on=[component]]: "Text with Unicode: \u{2192}";
```

For comprehensive string literal documentation, see [Literal Values and Data Types Specification](literal_values.md).

## 5. Complete Examples

### 5.1 Component Diagram with Comprehensive Notes

```filament
diagram component;

type Service = Rectangle [fill_color="lightblue", rounded=5];
type Database = Rectangle [fill_color="lightgreen", rounded=10];

api_gateway: Service;
auth_service: Service;
user_service: Service;
order_service: Service;
user_db: Database;
order_db: Database;

note [align="left"]: "Microservices Architecture - Production Environment";

note [on=[api_gateway], align="top"]: "Nginx + Kong API Gateway";
note [on=[api_gateway], align="right"]: "Handles rate limiting: 1000 req/min per client";

note [on=[auth_service], align="right"]: "JWT-based authentication with 1-hour expiry";

note [on=[user_service], align="bottom"]: "Manages user profiles and preferences";

note [on=[order_service], align="bottom"]: "Processes orders and payments via Stripe";

note [on=[user_db], align="left"]: "PostgreSQL 14 with read replicas";
note [on=[order_db], align="left"]: "PostgreSQL 14 with daily backups";

api_gateway -> auth_service;
api_gateway -> user_service;
api_gateway -> order_service;
user_service -> user_db;
order_service -> order_db;

note [align="right"]: "Last updated: 2024-01-15";
```

### 5.2 Sequence Diagram with All Note Types

```filament
diagram sequence;

client: Rectangle;
gateway: Rectangle;
auth_service: Rectangle;
user_service: Rectangle;
database: Rectangle;

note [align="left"]: "User Profile Update Flow";
note [align="right"]: "Average latency: 250ms";

client -> gateway: "PUT /users/profile";

note [on=[gateway]]: "Validates request format";

gateway -> auth_service: "Verify JWT token";

note [on=[auth_service]]: "Checks token signature and expiry";

auth_service -> gateway: "Token valid";

note [on=[gateway, user_service]]: "Forward authenticated request";

gateway -> user_service: "Update profile (user_id=123)";

activate user_service {
    note [on=[user_service]]: "Begin database transaction";

    user_service -> database: "BEGIN";
    user_service -> database: "UPDATE users SET ...";

    note [on=[user_service, database]]: "Validation and constraint checks";

    database -> user_service: "Success";
    user_service -> database: "COMMIT";

    note [on=[user_service]]: "Transaction completed";
};

user_service -> gateway: "Profile updated";
gateway -> client: "200 OK";

note [align="left"]: "Profile update successful";
```
