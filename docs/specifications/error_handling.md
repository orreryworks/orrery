# Filament Error Handling Specification

## 1. Overview

Filament provides a comprehensive error handling system designed to deliver precise, user-friendly error reporting that helps developers quickly identify and resolve issues in their diagram specifications. The system emphasizes accurate source location tracking, clear error messages, and actionable guidance.

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

### 3.1 Syntax Errors

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

The project includes a comprehensive collection of error examples in the `examples/errors/` directory. These examples demonstrate various error scenarios and can be used for:

- **Testing error handling**: Validate error message improvements
- **Learning common mistakes**: See typical syntax errors and their solutions
- **Development reference**: Examples for extending error handling capabilities

### 7.1 Available Examples

For a complete list of available error examples and their descriptions, see the comprehensive guide in [`examples/errors/README.md`](../../examples/errors/README.md). This documentation provides detailed information about each example file, its purpose, and the specific error scenarios it demonstrates.

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
