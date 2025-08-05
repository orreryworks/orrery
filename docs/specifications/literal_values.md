# Filament Literal Values and Data Types Specification

## 1. Overview

Filament supports two primary data types for attribute values: string literals and float literals. This specification provides comprehensive documentation on the syntax, usage, and behavior of these literal value types within the Filament language.

## 2. String Literals

String literals in Filament are enclosed in double quotes and support Rust-style escape sequences for enhanced text representation.

### 2.1 Basic String Syntax

```filament
"hello world"           // Basic string
"simple text"           // Alphanumeric content
""                      // Empty string
"with spaces and 123"   // Mixed content
```

### 2.2 Escape Sequences

Filament supports the following escape sequences within string literals:

#### Standard Escape Sequences
- `\"` - Double quote
- `\\` - Backslash
- `\/` - Forward slash
- `\n` - Newline
- `\r` - Carriage return
- `\t` - Tab
- `\b` - Backspace
- `\f` - Form feed
- `\0` - Null character

#### Examples
```filament
"quote: \"Hello World\""        // Contains quotes
"path\\to\\file"                // Windows path
"line 1\nline 2"               // Multi-line text
"tab\tdelimited\tdata"         // Tab-separated
```

### 2.3 Unicode Escape Sequences

Unicode characters can be included using the `\u{...}` syntax with 1-6 hexadecimal digits:

```filament
"emoji: \u{1F602}"             // 😂 (Face with Tears of Joy)
"symbol: \u{00AC}"             // ¬ (Not Sign)
"arrow: \u{2192}"              // → (Rightwards Arrow)
"math: \u{221E}"               // ∞ (Infinity)
```

#### Unicode Requirements
- Must use exactly 1-6 hexadecimal digits
- Must represent a valid Unicode code point
- Surrogate range (0xD800-0xDFFF) is not allowed
- Maximum value is 0x10FFFF

### 2.4 Escaped Whitespace

Whitespace immediately following a backslash at the end of a line is consumed, allowing for multi-line string formatting:

```filament
"This is a long string that spans \
 multiple lines but appears as one"
// Results in: "This is a long string that spans multiple lines but appears as one"
```

### 2.5 String Usage in Filament

String literals are used for:
- **Color values**: `"red"`, `"#ff0000"`, `"rgb(255,0,0)"`
- **Font families**: `"Arial"`, `"Helvetica"`, `"Courier New"`
- **Style values**: `"curved"`, `"orthogonal"`, `"straight"`
- **Display names and labels**: Component and relation labels
- **Background colors**: Diagram and component background colors

## 3. Float Literals

Float literals represent numeric values as 32-bit floating-point numbers (f32) and support multiple representation formats.

### 3.1 Standard Decimal Format

Basic decimal notation with explicit decimal point, including whole numbers:

```filament
2.5         // Standard decimal
10.0        // Explicit decimal point
0.75        // Leading zero
123.456     // Multiple decimal places
```

### 3.2 Whole Numbers

Whole numbers without decimal points are fully supported and treated as float literals:

```filament
1           // Equivalent to 1.0
17          // Equivalent to 17.0
42          // Equivalent to 42.0
1000        // Equivalent to 1000.0
```

Whole numbers provide a clean, readable syntax for integer-valued numeric attributes while maintaining the underlying float type system.

### 3.3 Abbreviated Decimal Formats

#### Leading Decimal Point
When the integer part is zero, it can be omitted:

```filament
.5          // Equivalent to 0.5
.25         // Equivalent to 0.25
.125        // Equivalent to 0.125
.001        // Equivalent to 0.001
```

#### Trailing Decimal Point
When the fractional part is zero, it can be omitted but the decimal point is required:

```filament
5.          // Equivalent to 5.0
100.        // Equivalent to 100.0
42.         // Equivalent to 42.0
```

### 3.4 Scientific Notation

Scientific notation uses `e` or `E` followed by an optional sign and exponent:

```filament
1e5         // 100000.0 (1 × 10⁵)
2.5e-3      // 0.0025 (2.5 × 10⁻³)
1.23e+4     // 12300.0 (1.23 × 10⁴)
1E5         // 100000.0 (uppercase E)
2.5E-3      // 0.0025 (uppercase E)
6.022e23    // Avogadro's number
```

#### Scientific Notation Rules
- Exponent can be positive or negative
- The `+` sign in the exponent is optional
- Both `e` and `E` are accepted
- The mantissa (number before e/E) can use any decimal format

### 3.4 Precision and Range

Float literals are stored as IEEE 754 single-precision floating-point numbers:
- **Precision**: Approximately 7 decimal digits
- **Range**: Approximately ±3.4 × 10³⁸
- **Smallest positive**: Approximately 1.2 × 10⁻³⁸

### 3.5 Float Usage in Filament

Float literals are used for:
- **Dimensions**: `line_width=2.5`, `rounded=10`
- **Text sizing**: `text=[font_size=16, padding=8.0]`
- **Positioning**: Coordinate and measurement values
- **Relation widths**: `width=2`

#### Examples by Attribute Type
```filament
// Shape attributes
component: Rectangle [
    line_width=2.5,     // Line thickness
    rounded=10,         // Corner radius (whole number)
];

// Text attributes
label: Rectangle [
    text=[font_size=16, padding=8.5]  // Nested text attributes
];

// Relation attributes
source -> [width=2] target;  // Relation line width (whole number)
```

## 4. Type Safety and Usage Rules

Filament enforces strict type safety for attribute values to prevent runtime errors and improve performance.

### 4.1 Strict Typing Rules

- **Numeric attributes** only accept float literals
- **Text attributes** only accept string literals
- **No automatic conversion** between string and numeric values
- **Compile-time validation** ensures type correctness

### 4.2 Correct Usage Examples

```filament
// ✅ Correct: String for colors, floats for dimensions
component: Rectangle [
    fill_color="blue",      // String literal
    line_width=2.5,         // Float literal
    rounded=10,             // Float literal
    text=[font_family="Arial", color="white"]  // Nested text attributes with color
];

// ✅ Correct: Mixed attribute types
type Database = Rectangle [
    fill_color="lightblue", // String
    rounded=10,             // Float
    line_width=2,           // Float
    text=[color="darkblue", font_size=14]  // Text with color
];

// ✅ Correct: Text colors with various formats
type ColorfulText = Rectangle [
    fill_color="white",
    text=[
        color="red",                    // Named color
        font_size=16,                   // Float
        background_color="#ffff00",     // Hex color
        padding=5.0                     // Float
    ]
];

// ✅ Correct: Semi-transparent text color
type TransparentText = Rectangle [
    fill_color="black",
    text=[color="rgba(255, 255, 255, 0.7)", font_size=18]  // Alpha transparency
];
```

### 4.3 Incorrect Usage Examples

```filament
// ❌ Incorrect: Using string for numeric attribute
component: Rectangle [
    line_width="2.5"        // Error: Expected float, found string
];

// ❌ Incorrect: Using float for string attribute
component: Rectangle [
    fill_color=255.0        // Error: Expected string, found float
];

// ❌ Incorrect: Using float for text color
component: Rectangle [
    text=[color=255.0]      // Error: Expected string, found float
];

// ❌ Incorrect: Using numeric value for text color
component: Rectangle [
    text=[color=16777215]   // Error: Expected string, found float
];
```

### 4.4 Attribute Type Reference

| Attribute | Type | Example Values |
|-----------|------|----------------|
| `fill_color` | String | `"red"`, `"#ff0000"` |
| `line_color` | String | `"blue"`, `"rgb(0,0,255)"` |
| `line_width` | Float | `2.0`, `1.5`, `.5`, `2` |
| `rounded` | Float | `10.0`, `5.`, `1e1`, `10` |
| `text` (group) | Nested | `[color="red", font_size=16, padding=8.0]` |
| `width` | Float | `2.0`, `3.5`, `2` (relations) |
| `color` | String | `"red"`, `"green"` (relations) |
| `style` | String | `"curved"`, `"orthogonal"` |
