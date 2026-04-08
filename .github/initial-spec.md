# SPECIFICATION: postamt CLI Briefmarken-Manager

## 1. DESIGN PRINCIPLES & CODESTYLE
- Project Name: postamt
- Language: Rust
- Comments: English, sparse, and concise. No emojis in output.
- XDG Compliance: ~/.config/postamt/, ~/.local/share/postamt/

## 2. DEPENDENCIES
- clap (v4, derive)
- rusqlite (bundled)
- serde & toml
- confy
- lopdf (for stamp extraction/cropping)
- printpdf (for envelope generation)
- sha2
- thiserror (for structured error handling)

## 3. PROGRAM SPECIFICATION

### A. STORAGE & INTEGRITY
- Duplicate Check: SHA-256 hash check before import.
- Import Logic: Copy/Move to vault. Automatically detect grid layout and extract stamps.
- Stamp Tracking: Track by ID and matrix_number (mandatory, format: A0 05BF A5BF A50E 00 0000 002B).
- Matrix Number Extraction: Automatically parse matrix code from PDF via text extraction.

### B. DATABASE SCHEMA
- Table 'imports': 
    - hash (PK, TEXT)
    - file_name (TEXT)
    - total_stamps (INTEGER)
    - created_at (TIMESTAMP)
- Table 'stamps': 
    - id (INTEGER, PK)
    - parent_hash (TEXT, FK)
    - stamp_index (INTEGER) -- position in grid (0-based)
    - matrix_number (TEXT, NOT NULL) -- extracted matrix code
    - printed_at (TIMESTAMP NULL) -- NULL = available, timestamp = printed

### C. CLI COMMANDS
- add <path> [--move]: Register PDF, auto-detect stamps, extract matrix codes, create DB entries.
- status: Show count of available stamps (WHERE printed_at IS NULL).
- profiles: List available profiles and indicate the current default.
- printers: List available printers with current default.
- reactivate <id|matrix_number>: Set printed_at back to NULL.
- print [--profile <name>] [--printer <printer>]:
    - If --profile is missing, use 'default_profile' from config.
    - If --printer is missing, use 'default_printer' from config.
    - Select oldest available stamp (ORDER BY id ASC WHERE printed_at IS NULL).
    - Extract stamp via lopdf (XObject/Form XObject approach).
    - Composite envelope via printpdf.
    - Execute printer command and set printed_at = now().

## 4. CONFIGURATION (config.toml)
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
grid_rows = 5
grid_cols = 2
# Defines the expected stamp arrangement in source PDF for automatic extraction.
# Tool will extract stamps based on this grid layout during import.

## 5. IMPLEMENTATION NOTES
- Use lopdf for precise cropping. The stamp should be treated as a reusable XObject.
- Matrix number extraction: Use lopdf text extraction to find patterns matching "A0 XXXX XXXX XXXX XX XXXX XXXX".
- 'reactivate' should be robust: search for ID first, then fallback to matrix_number string match.
- Ensure the config-loader provides a fallback if 'default_profile' or 'default_printer' points to non-existent entries.
- Error handling: Use thiserror for structured errors, convert to human-readable messages at CLI level.
- Printer detection: Use 'lpstat -p -d' for listing available printers (tab completion support).
