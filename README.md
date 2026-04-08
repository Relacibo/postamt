# postamt-rs

A CLI tool for managing and printing German postal stamps (Briefmarken) from PDF files.

## Features

- Import stamp PDFs with automatic matrix code extraction
- SHA-256 hash-based duplicate detection
- SQLite database for stamp tracking
- Support for multiple envelope profiles (DL, C6)
- Dry-run mode for testing
- Mark stamps as used/available
- XDG-compliant storage

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/postamt-rs`.

## Usage

### Import stamps from PDF

```bash
# Copy PDF to vault
postamt-rs add path/to/stamps.pdf

# Move PDF to vault
postamt-rs add path/to/stamps.pdf --move
```

### Check available stamps

```bash
postamt-rs status
```

### List envelope profiles

```bash
postamt-rs profiles
```

### List available printers

```bash
postamt-rs printers
```

### Mark stamps

```bash
# Mark stamp as used (by ID or matrix number)
postamt-rs mark-used 1
postamt-rs mark-used "A0 05BF A50E 00 0000 002B"

# Mark stamp as available again
postamt-rs mark-available 1
postamt-rs mark-available "A0 05BF A50E 00 0000 002B"
```

### Print envelope

```bash
# Dry run (saves PDF to ./dry-runs/)
postamt-rs print --dry-run

# Print to default printer with default profile
postamt-rs print

# Print with specific profile and printer
postamt-rs print --profile C6 --printer my-printer
```

## Configuration

Configuration is stored in `~/.config/postamt/config.toml` (auto-generated on first run).

Default configuration:
```toml
default_profile = "DL"
default_printer = "lpr"

[import]
default_action = "copy"

[[profiles]]
name = "DL"
width = 220.0
height = 110.0
offset_stamp_x = 180.0
offset_stamp_y = 10.0

[[profiles]]
name = "C6"
width = 162.0
height = 114.0
offset_stamp_x = 130.0
offset_stamp_y = 10.0

[layout]
grid_cols = 4
grid_rows_max = 8
```

## Storage

- Database: `~/.local/share/postamt/postamt.db`
- Vault: `~/.local/share/postamt/vault/`
- Dry-run output: `./dry-runs/`

## Requirements

- Single-page PDFs only
- Matrix code format: `A0 XXXX XXXX XX XXXX XXXX`
- Grid layout: 4 columns, up to 8 rows
- Stamp value: 0.95€ (hardcoded in v1)

## Development

See `.github/initial-spec.md` for the complete specification.

## License

See LICENSE file.
