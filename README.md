# postamt-rs

Command-line tool for managing and printing digital stamps from Deutsche Post.

## Features

- Import stamp PDFs and automatically extract individual stamps
- Track used/available stamps in local database
- Print stamps on envelopes with configurable profiles
- Support for different envelope sizes (DL, C6, etc.)
- Interactive printer selection with configuration management

## Installation

```bash
cargo install --path .
```

### Shell Completions

Generate shell completions for tab completion:

```bash
# For Bash
postamt completions bash > ~/.local/share/bash-completion/completions/postamt

# For Zsh
postamt completions zsh > ~/.zsh/completions/_postamt

# For Fish
postamt completions fish > ~/.config/fish/completions/postamt.fish
```

## Quick Start

1. Import a stamp sheet:
   ```bash
   postamt add stamps.pdf
   ```

2. Configure your printer (optional):
   ```bash
   postamt config default_printer Brother_HL_L2400DW
   ```

3. Print a stamp:
   ```bash
   postamt print
   ```

## Commands

### `add` - Import stamps
```bash
# Copy PDF to vault and extract stamps
postamt add stamps.pdf

# Move instead of copy
postamt add stamps.pdf --move
```

Automatically detects:
- Grid layout stamps (e.g., TestPrint.pdf with 4 columns)
- Single stamps with matrix codes in filename
- Generated stamp sheets from `gen-stamps` tool

### `print` - Print stamps
```bash
# Print next available stamp (interactive selection)
postamt print

# Print specific stamp by matrix code
postamt print A0_05BF_A50E_00_0000_002B

# Dry run (save PDF to ./dry-runs/ instead of printing)
postamt print --dry-run

# Use specific envelope profile
postamt print --profile C6

# Override configured printer
postamt print --printer MyPrinter
```

### `list` - List stamps
```bash
# List all stamps and files
postamt list

# Filter by file hash
postamt list --file a1b2c3d4...

# Show only available stamps
postamt list --available

# Show only used stamps
postamt list --used

# Output as JSON
postamt list --format json
```

### `printers` - List available printers
```bash
postamt printers
```

Shows all system printers detected by `lpstat -e`, with markers for system default and configured default.

### `config` - Manage configuration
```bash
# Show entire configuration
postamt config

# Get a specific value
postamt config default_printer
postamt config profiles.DL.width

# Set a value
postamt config default_printer Brother_HL_L2400DW
postamt config default_profile C6
postamt config import.default_action move

# Set nested values (profiles)
postamt config profiles.DL.offset_stamp_x 175.0
```

Configuration uses dot notation for nested values. All configuration is stored in `~/.config/postamt/config.toml`.

## Configuration

### Config File Location
`~/.config/postamt/config.toml`

If no config file exists, built-in defaults are used.

### Config Structure
```toml
default_profile = "DL"
default_printer = "Brother_HL_L2400DW"  # optional

[import]
default_action = "copy"  # or "move"

# Built-in profiles (can be extended with custom profiles)
[profiles.DL]
width = 220.0
height = 110.0
offset_stamp_x = 170.0
offset_stamp_y = 8.0

[profiles.C6]
width = 162.0
height = 114.0
offset_stamp_x = 130.0
offset_stamp_y = 10.0
```

**Note:** The grid layout (4 columns × 8 rows max) is hardcoded for Deutsche Post stamp sheets and cannot be configured.

### Profile Management
- Built-in profiles (`DL`, `C6`) are always available
- Add custom profiles in config file
- Custom profiles override built-in profiles with same name
- Use `postamt config profiles.<name>.<field> <value>` to modify

### Printer Selection Priority
1. CLI argument: `--printer MyPrinter`
2. Config default: `default_printer = "MyPrinter"`
3. Interactive prompt if none configured

## Data Storage

- **Database**: `~/.local/share/postamt/stamps.db` (SQLite)
- **Vault**: `~/.local/share/postamt/vault/` (Original PDFs)
- **Config**: `~/.config/postamt/config.toml`

## Development Tools

### `gen-stamps` - Generate test stamp sheets
Located in `tools/gen-stamps/`:

```bash
cd tools/gen-stamps
cargo run -- output.pdf
```

Generates stamp sheets with random matrix codes for testing. Randomly selects between TestPrint.pdf (2 rows) and TestPrint-full.pdf (8 rows) as template, replacing all `X` placeholders with random alphanumeric characters.

## Example Workflow

```bash
# 1. Generate test stamps
cd tools/gen-stamps && cargo run -- ../../test-stamps.pdf && cd ../..

# 2. Import them
postamt add test-stamps.pdf

# 3. List available stamps
postamt list --available

# 4. Configure printer (one-time)
postamt config default_printer Brother_HL_L2400DW

# 5. Print stamps as needed
postamt print                    # Interactive selection
postamt print A0_05BF_...        # Specific stamp
postamt print --dry-run          # Test without printing

# 6. Check what's been used
postamt list --used
```

## Grid Layout Coordinates

The stamp extraction uses precise calibration based on the Deutsche Post grid layout:
- Grid crosses mark exact stamp corners
- Coordinates are measured from bottom-left (PDF coordinate system)
- See `.github/initial-spec.md` for exact grid positions

Reference PDFs in `assets/pdfs/example-pdfs/`:
- `Briefmarken.1Stk.07.04.2026_1916.pdf` - Single stamp example
- `TestPrint.pdf` - 4×2 grid for testing
- `TestPrint-full.pdf` - 4×8 full grid

## License

See LICENSE file.

