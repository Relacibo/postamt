use rusqlite::Connection;
use std::path::Path;
use crate::config::Config;
use crate::db;
use crate::error::{Error, Result};
use crate::pdf;
use crate::printer;

pub fn add_stamps(conn: &Connection, config: &Config, path: &str, should_move: bool) -> Result<()> {
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
    
    // Extract matrix codes
    let matrix_codes = pdf::extract_matrix_codes(
        source_path,
        config.layout.grid_cols,
        config.layout.grid_rows_max,
    )?;
    
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
    let file_name = source_path.file_name()
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
    profile_name: Option<&str>,
    printer_name: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    // Get oldest available stamp
    let stamp = match db::get_oldest_available_stamp(conn)? {
        Some(s) => s,
        None => {
            let count = db::count_available_stamps(conn)?;
            return Err(Error::Custom(format!("No stamps available (current count: {})", count)));
        }
    };
    
    // Determine profile
    let profile_name = profile_name.unwrap_or(&config.default_profile);
    let profile = crate::config::get_profile(config, profile_name)
        .ok_or_else(|| Error::Custom(format!("Profile not found: {}", profile_name)))?;
    
    // Determine printer
    let printer = printer_name.unwrap_or(&config.default_printer);
    
    // Load source PDF from vault
    let vault_path = db::get_vault_path()?;
    let source_pdf = vault_path.join(format!("{}.pdf", stamp.parent_hash));
    
    // Extract stamp
    let stamp_data = pdf::extract_stamp(&source_pdf, stamp.stamp_index as usize, config.layout.grid_cols)?;
    
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
        
        let output_path = dry_runs_dir.join(format!("envelope-{}.pdf", stamp.matrix_number.replace(' ', "_")));
        std::fs::write(&output_path, envelope_data)?;
        
        println!("Dry run: saved to {}", output_path.display());
    } else {
        // Create temporary file for printing
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("postamt-{}.pdf", stamp.id));
        std::fs::write(&temp_path, envelope_data)?;
        
        // Print
        printer::print_pdf(printer, temp_path.to_str().unwrap())?;
        
        // Mark as printed
        db::mark_stamp_printed_by_id(conn, stamp.id)?;
        
        // Clean up
        let _ = std::fs::remove_file(temp_path);
        
        println!("Printed stamp {} to {}", stamp.matrix_number, printer);
    }
    
    Ok(())
}
