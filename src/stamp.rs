use crate::config::Config;
use crate::db;
use crate::error::{Error, Result};
use crate::pdf;
use crate::printer;
use rusqlite::Connection;
use std::path::Path;

pub fn add_stamps(conn: &Connection, path: &str, should_move: bool) -> Result<()> {
    let source_path = Path::new(path);

    if !source_path.exists() {
        return Err(Error::Custom(format!("File not found: {}", path)));
    }

    // Compute hash
    let hash = pdf::compute_hash(source_path)?;

    // Check for duplicates
    if db::hash_exists(conn, &hash)? {
        println!("PDF already imported");
        return Ok(());
    }

    // Try to extract matrix codes from filename first (for generated-*.pdf)
    let file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let matrix_codes = if file_name.starts_with("generated-") {
        // Extract matrix code from filename: generated-XXXX_XXXX_XXXX_XX_XXXX_XXXXX.pdf
        if let Some(matrix_part) = file_name
            .strip_prefix("generated-")
            .and_then(|s| s.strip_suffix(".pdf"))
        {
            let matrix_code = matrix_part.replace("_", " ");
            // For generated stamps, duplicate the matrix code for all grid positions
            // Assume full grid: 4 cols x 8 rows = 32 stamps
            vec![matrix_code; 32]
        } else {
            // Fallback to grid extraction
            pdf::extract_matrix_codes(source_path)?
        }
    } else {
        // Standard grid extraction
        pdf::extract_matrix_codes(source_path)?
    };

    if matrix_codes.is_empty() {
        return Err(Error::Custom("No valid stamps found in PDF".to_string()));
    }

    // Copy or move to vault
    let vault_path = db::get_vault_path()?;
    let dest_path = vault_path.join(format!("{}.pdf", hash));

    if should_move {
        std::fs::rename(source_path, &dest_path)?;
    } else {
        std::fs::copy(source_path, &dest_path)?;
    }

    // Insert into database
    let file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    db::insert_import(conn, &hash, file_name, matrix_codes.len())?;

    for (index, matrix_number) in matrix_codes.iter().enumerate() {
        db::insert_stamp(conn, &hash, index, matrix_number)?;
    }

    println!("Imported {} stamps from {}", matrix_codes.len(), file_name);
    Ok(())
}

pub fn show_status(conn: &Connection) -> Result<()> {
    let count = db::count_available_stamps(conn)?;
    println!("Available stamps: {}", count);
    Ok(())
}

pub fn mark_available(conn: &Connection, identifier: &str) -> Result<()> {
    if db::mark_stamp_available(conn, identifier)? {
        println!("Stamp marked as available");
    } else {
        println!("Stamp not found");
    }
    Ok(())
}

pub fn mark_used(conn: &Connection, identifier: &str) -> Result<()> {
    if db::mark_stamp_used(conn, identifier)? {
        println!("Stamp marked as used");
    } else {
        println!("Stamp not found");
    }
    Ok(())
}

pub fn print_stamp(
    conn: &Connection,
    config: &Config,
    identifier: Option<&str>,
    profile_name: Option<&str>,
    printer_name: Option<&str>,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    // Get stamp - either specific one or oldest available
    let stamp = if let Some(id) = identifier {
        let stamp = db::get_stamp_by_id(conn, id)?
            .ok_or_else(|| Error::Custom(format!("Stamp not found: {}", id)))?;
        
        if stamp.used && !force {
            return Err(Error::Custom(format!(
                "Stamp {} is already marked as used. Use --force to print anyway.",
                id
            )));
        }
        
        if stamp.used && force {
            eprintln!("⚠️  Warning: Printing stamp that is already marked as used!");
        }
        
        stamp
    } else {
        // Get oldest available stamp
        match db::get_oldest_available_stamp(conn)? {
            Some(s) => s,
            None => {
                let count = db::count_available_stamps(conn)?;
                return Err(Error::Custom(format!(
                    "No stamps available (current count: {})",
                    count
                )));
            }
        }
    };

    // Determine profile
    let profile_name = profile_name.unwrap_or(&config.default_profile);
    let profile = crate::config::get_profile(config, profile_name)
        .ok_or_else(|| Error::Custom(format!("Profile not found: {}", profile_name)))?;

    // Load source PDF from vault
    let vault_path = db::get_vault_path()?;
    let source_pdf = vault_path.join(format!("{}.pdf", stamp.parent_hash));

    // Extract stamp
    let stamp_data = pdf::extract_stamp(&source_pdf, stamp.stamp_index as usize)?;

    // Create envelope
    let envelope_data = pdf::create_envelope(
        profile.width,
        profile.height,
        profile.offset_stamp_x,
        profile.offset_stamp_y,
        &stamp_data,
    )?;

    if dry_run {
        // Save to dry-runs directory
        let dry_runs_dir = Path::new("./dry-runs");
        std::fs::create_dir_all(dry_runs_dir)?;

        let output_path = dry_runs_dir.join(format!(
            "envelope-{}.pdf",
            stamp.matrix_number.replace(' ', "_")
        ));
        std::fs::write(&output_path, envelope_data)?;

        println!("Dry run: saved to {}", output_path.display());
    } else {
        // Resolve printer (may prompt user)
        let printer = match printer::resolve_printer(config, printer_name)? {
            Some(p) => p,
            None => {
                println!("Cancelled.");
                return Ok(());
            }
        };

        // Create temporary file for printing
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("postamt-{}.pdf", stamp.id));
        std::fs::write(&temp_path, envelope_data)?;

        // Print
        printer::print_pdf(&printer, temp_path.to_str().unwrap())?;

        // Mark as printed
        db::mark_stamp_printed_by_id(conn, stamp.id)?;

        // Clean up
        let _ = std::fs::remove_file(temp_path);

        println!("Printed stamp {} to {}", stamp.matrix_number, printer);
    }

    Ok(())
}

pub fn list_stamps(
    conn: &Connection,
    file_filter: Option<&str>,
    available_only: bool,
    used_only: bool,
    format: crate::OutputFormat,
) -> Result<()> {
    use crate::OutputFormat;
    use serde::Serialize;

    #[derive(Serialize)]
    struct StampOutput {
        id: i64,
        index: i64,
        matrix: String,
        status: String,
        printed_at: Option<String>,
    }

    #[derive(Serialize)]
    struct FileOutput {
        hash: String,
        file_name: String,
        created_at: String,
        total_stamps: i64,
        stamps: Vec<StampOutput>,
    }

    let imports = db::get_imports(conn, file_filter)?;
    let mut files: Vec<FileOutput> = Vec::new();

    for import in imports {
        let mut stamps = db::get_stamps_for_import(conn, &import.hash, available_only, used_only)?;

        // Skip files with no matching stamps
        if stamps.is_empty() && (available_only || used_only) {
            continue;
        }

        // Sort stamps by id (oldest first)
        stamps.sort_by_key(|s| s.id);

        let stamp_outputs: Vec<StampOutput> = stamps
            .iter()
            .map(|s| StampOutput {
                id: s.id,
                index: s.stamp_index,
                matrix: s.matrix_number.clone(),
                status: if s.printed_at.is_some() {
                    "used".to_string()
                } else {
                    "available".to_string()
                },
                printed_at: s.printed_at.clone(),
            })
            .collect();

        files.push(FileOutput {
            hash: import.hash,
            file_name: import.file_name,
            created_at: import.created_at,
            total_stamps: import.total_stamps,
            stamps: stamp_outputs,
        });
    }

    // Sort files by created_at (oldest first) using timestamp parsing
    files.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    match format {
        OutputFormat::Toml => {
            #[derive(Serialize)]
            struct Root {
                files: Vec<FileOutput>,
            }
            let output = toml::to_string_pretty(&Root { files })
                .map_err(|e| Error::Custom(format!("TOML serialize error: {}", e)))?;
            println!("{}", output);
        }
        OutputFormat::Json => {
            let output = serde_json::to_string_pretty(&files)
                .map_err(|e| Error::Custom(format!("JSON serialize error: {}", e)))?;
            println!("{}", output);
        }
    }

    Ok(())
}
