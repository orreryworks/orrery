# Filament Error Handling Specification

## 1. Overview

Filament provides an error handling system for precise, user-friendly error reporting that helps developers quickly identify and resolve issues in their diagram specifications. The system emphasizes accurate source location tracking, clear error messages, and actionable guidance.

## 2. Error Handling Architecture

### 2.1 Core Components

The error handling system provides the following capabilities:

- **Error Detection** - Identifies syntax and semantic errors during compilation
- **Message Formatting** - Transforms technical errors into clear, user-friendly messages
- **Location Tracking** - Maintains precise source location information for accurate error reporting
- **Visual Presentation** - Formats errors with source highlighting and professional display

### 2.2 Error Processing Requirements

The system must process errors through these stages:

1. **Error Identification** - Detect and classify errors during compilation
2. **Location Preservation** - Maintain accurate source position information
3. **Message Generation** - Create clear, actionable error descriptions
4. **Visual Formatting** - Present errors with proper highlighting and context
5. **User Display** - Deliver formatted errors to the user interface

## 3. Error Categories

### 10.3 Syntax Errors (Blocks and Explicit)

Errors detected during the parsing phase when source code doesn't conform to Filament's grammar:

- **Missing semicolons** - Statements not properly terminated
- **Missing colons** - Component definitions lacking proper syntax
- **Missing brackets/braces** - Unmatched delimiters in attributes and blocks
- **Invalid tokens** - Unrecognized characters or token sequences
- **Unexpected end of input** - Incomplete statements or structures

### 3.2 Semantic Errors

Errors detected during elaboration when syntax is valid but semantics are incorrect:

- **Undefined type references** - References to types that don't exist
- **Undefined component references** - Components used in relations but not declared
- **Invalid attribute values** - Attribute values that don't match expected formats
- **Type system violations** - Inconsistent or incompatible type usage
- **Activate block diagram type errors** - Activate blocks used in component diagrams (only allowed in sequence diagrams)
- **Activate block component errors** - Activate blocks referencing undefined components
- **Explicit activation diagram type errors** - `activate`/`deactivate` statements used in component diagrams (only allowed in sequence diagrams)
- **Activation pairing violations** - Unpaired `activate`/`deactivate` statements or out-of-order deactivation before activation

## 4. Error Message Format

### 4.1 Standard Error Structure

All error messages follow a consistent format:

```
Parse error: [clear, descriptive message]
   ╭─[line:column]
 n │ [source line with context]
   │ [visual highlighting pointing to error location]
   ╰────
help: [actionable guidance and common solutions]
```

### 4.2 Message Components

- **Error Type**: "Parse error" for syntax issues, specific type for semantic errors
- **Descriptive Message**: Clear, non-technical explanation of what went wrong
- **Location Indicator**: Precise line and column numbers
- **Source Context**: Relevant source lines with visual highlighting
- **Help Text**: Actionable suggestions for resolving the issue

## 5. Location Accuracy and Span Tracking

### 5.1 Character-Perfect Positioning

The error system provides character-level accuracy for error locations:

- Errors point to the exact character position where the issue occurs
- Span information is preserved throughout the parsing pipeline
- Multi-character identifiers are highlighted with precise start/end positions

### 5.2 Visual Highlighting

Errors use visual indicators to show exact locations:

- `╭─▶` and `├─▶` arrows point to error locations
- `╰────` underlines highlight the problematic text
- Line numbers provide context for navigation

## 6. Error Message Examples

### 6.1 Missing Semicolon

```
Parse error: missing semicolon
   ╭─[3:19]
 2 │
 3 │ ╭─▶ frontend: Rectangle
 4 │ │
 5 │ ├─▶ backend: Rectangle;
   · ╰──── here
 6 │
   ╰────
help: Common syntax issues include:
      • Missing semicolon ';' after statements
      • Missing colon ':' in component definitions (use 'name: Type;')
      • Unmatched brackets '[', ']', '{', '}'
      • Invalid relation syntax (use 'source -> target;')
```

### 6.2 Undefined Type Reference

```
× Base type 'rectangle' not found
  ╭─[10:19]
9 │ // Error: Typo in built-in type (should be Rectangle)
10│ type ApiService = rectangle [fill_color="green"];
  ·                   ────┬────
  ·                       ╰── undefined type
```

### 6.3 Missing Bracket

```
Parse error: missing closing bracket ']'
   ╭─[4:25]
 3 │
 4 │ ╭─▶ component: Rectangle [color="red"
 5 │ │
 6 │ ├─▶ other: Rectangle;
   · ╰──── here
 7 │
   ╰────
help: Common syntax issues include:
      • Missing semicolon ';' after statements
      • Missing colon ':' in component definitions (use 'name: Type;')
      • Unmatched brackets '[', ']', '{', '}'
      • Invalid relation syntax (use 'source -> target;')
```

## 7. Error Examples Repository

The project includes a collection of error examples in the `examples/errors/` directory. These examples demonstrate various error scenarios and can be used for:

- **Testing error handling**: Validate error message improvements
- **Learning common mistakes**: See typical syntax errors and their solutions
- **Development reference**: Examples for extending error handling capabilities

### 7.1 Available Examples

For a complete list of available error examples and their descriptions, see [`examples/errors/README.md`](../../examples/errors/README.md). This documentation provides detailed information about each example file, its purpose, and the specific error scenarios it demonstrates.

### 7.2 Usage

Test any error example with:
```bash
cargo run examples/errors/[example_file.fil]
```

## 8. Help Text and Guidance

### 8.1 Common Issues

The help system provides guidance for frequently encountered problems:

- **Semicolon issues**: Explains proper statement termination
- **Colon syntax**: Shows correct component definition format
- **Bracket matching**: Identifies unmatched delimiters
- **Relation syntax**: Demonstrates proper relation format

### 8.2 Contextual Suggestions

Help text is tailored to provide relevant suggestions based on the error type and context, helping users understand not just what went wrong but how to fix it.

## 9. Error Handling Standards

### 9.1 Message Quality

All error messages must be:
- **Clear**: Use plain language, avoid technical jargon
- **Specific**: Point to exact problems, not generic issues
- **Actionable**: Provide concrete steps for resolution
- **Consistent**: Follow the standard format and style

### 9.2 Location Accuracy

All error locations must be:
- **Character-precise**: Point to exact character positions
- **Contextually relevant**: Show meaningful source context
- **Visually clear**: Use consistent highlighting patterns
- **Properly formatted**: Follow the standard visual format

### 10. Activation Error Examples (Blocks and Explicit Statements)

### 10.1 Diagram Type Validation (Blocks and Explicit)

**Error**: Using activation in component diagrams (block or explicit)

```filament
diagram component;
user: Rectangle;
server: Rectangle;

// Block form (error in component diagrams)
activate user {  // Error
    user -> server: "Request";
};

// Explicit form (error in component diagrams)
activate user;    // Error
deactivate user;  // Error
```

**Error Message**:
```
Error: Activation is only supported in sequence diagrams
  --> example.filament:5:1
   |
 5 | activate user {
   | ^^^^^^^^ activation not allowed here
   |
   = help: Activation is used for temporal grouping in sequence diagrams
   = note: Component diagrams use curly braces for embedded diagrams only
```

### 10.2 Component Reference Validation (Blocks and Explicit)

**Error**: Referencing non-existent components in activation (block or explicit)

```filament
diagram sequence;
user: Rectangle;

// Block form
activate server {  // Error: 'server' component not defined
    user -> server: "Request";
};

// Explicit form
activate server;   // Error: 'server' component not defined

// Explicit form
activate server;   // Error: 'server' component not defined
```

**Error Message**:
```
Error: Component 'server' is not defined
  --> example.filament:4:10
   |
 4 | activate server {
   |          ^^^^^^ undefined component
   |
   = help: Define the component before using it in an activate block
   = note: Available components: user
```

### 10.3 Syntax Errors (Blocks and Explicit)

**Block form**: Missing semicolon after activate block

```filament
diagram sequence;
user: Rectangle;
server: Rectangle;

activate user {
    user -> server: "Request";
}  // Error: missing semicolon
```

**Error Message**:
```
Error: Expected ';' after activate block
  --> example.filament:7:1
   |
 7 | }
   |  ^ expected ';'
   |
   = help: Activate blocks must be terminated with a semicolon
   = note: All top-level elements require semicolon termination
```

### 10.4 Explicit Statement Errors

Explicit activation statements provide fine-grained control over lifeline activation timing. Errors related to these statements include diagram-type violations and pairing/order issues.

#### 10.4.1 Diagram Type Validation (Explicit Statements)

```
Error: Activate statements are only supported in sequence diagrams
  --> example.filament:3:1
   |
 3 | activate user;
   | ^^^^^^^^ not allowed here
   |
   = help: Explicit activate/deactivate statements are sequence-diagram features
   = note: Use sequence diagrams for temporal grouping; component diagrams do not support activation
```

#### 10.4.2 Deactivate Before Activate

```
Error: Deactivate without matching activate
  --> example.filament:6:1
   |
 6 | deactivate user;
   | ^^^^^^^^^^ no prior activation for 'user'
   |
   = help: Ensure each deactivate matches a preceding activate for the same component
```

#### 10.4.3 Unpaired Activate at End of Scope

```
Error: Unpaired activate at end of scope
  --> example.filament:4:1
   |
 4 | activate server;
   | ^^^^^^^^ activation not closed by a matching deactivate
   |
   = help: Add a 'deactivate server;' before the end of the current scope
```
