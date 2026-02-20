# Orrery CLI

Command-line interface for the Orrery diagram language.

## Installation

Install from source:

```bash
cargo install --path .
```

Or from crates.io:

```bash
cargo install orrery-cli
```

## Usage

```bash
orrery <input.orr> [OPTIONS]
```

### Basic Example

```bash
# Render a diagram to SVG
orrery diagram.orr -o output.svg

# With custom configuration
orrery diagram.orr -o output.svg --config custom.toml

# With debug logging
orrery diagram.orr -o output.svg --log-level debug
```

### Command-Line Options

```
Arguments:
  <INPUT>  Path to the input Orrery file

Options:
  -o, --output <OUTPUT>        Path to output SVG file [default: out.svg]
  -c, --config <CONFIG>        Path to configuration file (TOML)
      --log-level <LOG_LEVEL>  Log level (off, error, warn, info, debug, trace) [default: info]
  -h, --help                   Print help
  -V, --version                Print version
```

## Configuration

The CLI searches for configuration files in this order:

1. Path specified with `--config` flag
2. `orrery/config.toml` in current directory
3. Platform-specific config directory:
   - Linux: `~/.config/orrery/config.toml`
   - macOS: `~/Library/Application Support/com.orrery.orrery/config.toml`
   - Windows: `%APPDATA%\orrery\orrery\config.toml`

### Example Configuration

```toml
[layout]
component = "sugiyama"
sequence = "basic"

[style]
background_color = "#ffffff"

[style.lifeline]
color = "#000000"
width = 2.0
style = "solid"
```

## Example Diagrams

See the [examples directory](../../../examples/) for sample `.orr` files.

Process an example:

```bash
orrery examples/shape_types_showcase.orr -o showcase.svg
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
