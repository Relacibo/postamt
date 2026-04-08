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
- Vault Location: ~/.local/share/postamt/vault/
- Database Location: ~/.local/share/postamt/postamt.db
- Storage Format: {sha256_hash}.pdf (original file stored in vault)
- DB stores only hash reference + metadata
- Stamp Tracking: Track by ID and matrix_number (mandatory, format: A0 XXXX XXXX XX XXXX XXXX).
- Matrix Number Extraction: Automatically parse matrix code from PDF via text extraction.
- Stamp Value: 0.95€ (95 ct) - only this value supported in v1.
- PDF Constraints: Only single-page PDFs supported. Last row may be incomplete (less than 4 stamps).

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
    - If duplicate (hash exists): Show message "PDF already imported" and exit.
- status: Show count of available stamps (WHERE printed_at IS NULL).
- profiles: List available profiles and indicate the current default.
- printers: List available printers with current default.
    - If no printers found: Show message "No printers found".
- mark-available <id|matrix_number>: Set printed_at back to NULL.
- mark-used <id|matrix_number>: Set printed_at = now() without actually printing.
- print [--profile <name>] [--printer <printer>] [--dry-run]:
    - If --profile is missing, use 'default_profile' from config.
    - If --printer is missing, use 'default_printer' from config.
    - Select oldest available stamp (ORDER BY id ASC WHERE printed_at IS NULL).
    - If no stamps available: Show error message with current count (0).
    - Extract stamp via lopdf (XObject/Form XObject approach).
    - Composite envelope via printpdf.
    - If --dry-run: Save PDF to ./dry-runs/envelope-{matrix_number}.pdf, don't execute printer, don't mark as printed.
    - Else: Execute printer command and set printed_at = now().

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
grid_cols = 4
grid_rows_max = 8
# Defines the expected stamp arrangement in source PDF for automatic extraction.
# Grid: 4 columns, up to 8 rows maximum.
# Tool will detect actual number of stamps based on this grid layout during import.

## 5. IMPLEMENTATION NOTES
- Use lopdf for precise cropping. The stamp should be treated as a reusable XObject.
- Matrix number extraction: Use lopdf text extraction to find patterns matching "A0 XXXX XXXX XX XXXX XXXX" (exact format).
- Grid detection: Assume 4 columns, detect rows (max 8) based on content during import.
- Stamp dimensions and grid coordinates: Static in code, derived from TestPrint-full.pdf reference layout.
- 'mark-available' and 'mark-used' should be robust: search for ID first, then fallback to matrix_number string match.
- Ensure the config-loader provides a fallback if 'default_profile' or 'default_printer' points to non-existent entries.
- Error handling: Use thiserror for structured errors, convert to human-readable messages at CLI level.
- Printer detection: Use 'lpstat -p -d' for listing available printers (tab completion support).
- Storage: Original PDFs stored in vault, referenced by hash. No modification to source files.
- Dry-run output: Create ./dry-runs/ directory if not exists, add to .gitignore.
