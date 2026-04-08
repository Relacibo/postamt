use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use crate::error::{Error, Result};

pub fn get_db_path() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| Error::Custom("Could not determine data directory".to_string()))?;
    let postamt_dir = data_dir.join("postamt");
    std::fs::create_dir_all(&postamt_dir)?;
    Ok(postamt_dir.join("postamt.db"))
}

pub fn get_vault_path() -> Result<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| Error::Custom("Could not determine data directory".to_string()))?;
    let vault_dir = data_dir.join("postamt").join("vault");
    std::fs::create_dir_all(&vault_dir)?;
    Ok(vault_dir)
}

pub fn init(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS imports (
            hash TEXT PRIMARY KEY,
            file_name TEXT NOT NULL,
            total_stamps INTEGER NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS stamps (
            id INTEGER PRIMARY KEY,
            parent_hash TEXT NOT NULL,
            stamp_index INTEGER NOT NULL,
            matrix_number TEXT NOT NULL,
            printed_at TIMESTAMP NULL,
            FOREIGN KEY(parent_hash) REFERENCES imports(hash)
        )",
        [],
    )?;
    
    Ok(conn)
}

pub fn hash_exists(conn: &Connection, hash: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM imports WHERE hash = ?1",
        params![hash],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn insert_import(conn: &Connection, hash: &str, file_name: &str, total_stamps: usize) -> Result<()> {
    conn.execute(
        "INSERT INTO imports (hash, file_name, total_stamps) VALUES (?1, ?2, ?3)",
        params![hash, file_name, total_stamps as i64],
    )?;
    Ok(())
}

pub fn insert_stamp(conn: &Connection, parent_hash: &str, stamp_index: usize, matrix_number: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO stamps (parent_hash, stamp_index, matrix_number) VALUES (?1, ?2, ?3)",
        params![parent_hash, stamp_index as i64, matrix_number],
    )?;
    Ok(())
}

pub fn count_available_stamps(conn: &Connection) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stamps WHERE printed_at IS NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}

pub fn mark_stamp_available(conn: &Connection, identifier: &str) -> Result<bool> {
    if let Ok(id) = identifier.parse::<i64>() {
        let rows = conn.execute(
            "UPDATE stamps SET printed_at = NULL WHERE id = ?1",
            params![id],
        )?;
        return Ok(rows > 0);
    }
    
    let rows = conn.execute(
        "UPDATE stamps SET printed_at = NULL WHERE matrix_number = ?1",
        params![identifier],
    )?;
    Ok(rows > 0)
}

pub fn mark_stamp_used(conn: &Connection, identifier: &str) -> Result<bool> {
    if let Ok(id) = identifier.parse::<i64>() {
        let rows = conn.execute(
            "UPDATE stamps SET printed_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id],
        )?;
        return Ok(rows > 0);
    }
    
    let rows = conn.execute(
        "UPDATE stamps SET printed_at = CURRENT_TIMESTAMP WHERE matrix_number = ?1",
        params![identifier],
    )?;
    Ok(rows > 0)
}

pub struct StampRecord {
    pub id: i64,
    pub parent_hash: String,
    pub stamp_index: i64,
    pub matrix_number: String,
}

pub fn get_oldest_available_stamp(conn: &Connection) -> Result<Option<StampRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_hash, stamp_index, matrix_number 
         FROM stamps 
         WHERE printed_at IS NULL 
         ORDER BY id ASC 
         LIMIT 1"
    )?;
    
    let mut rows = stmt.query([])?;
    
    if let Some(row) = rows.next()? {
        Ok(Some(StampRecord {
            id: row.get(0)?,
            parent_hash: row.get(1)?,
            stamp_index: row.get(2)?,
            matrix_number: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn mark_stamp_printed_by_id(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE stamps SET printed_at = CURRENT_TIMESTAMP WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub struct ImportRecord {
    pub hash: String,
    pub file_name: String,
    pub total_stamps: i64,
    pub created_at: String,
}

pub struct StampInfo {
    pub id: i64,
    pub stamp_index: i64,
    pub matrix_number: String,
    pub printed_at: Option<String>,
}

pub fn get_imports(conn: &Connection, file_filter: Option<&str>) -> Result<Vec<ImportRecord>> {
    let mut result = Vec::new();
    
    if let Some(filter) = file_filter {
        let pattern = format!("%{}%", filter);
        let mut stmt = conn.prepare(
            "SELECT hash, file_name, total_stamps, created_at FROM imports WHERE hash LIKE ?1 OR file_name LIKE ?1 ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(ImportRecord {
                hash: row.get(0)?,
                file_name: row.get(1)?,
                total_stamps: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        for row in rows {
            result.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT hash, file_name, total_stamps, created_at FROM imports ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ImportRecord {
                hash: row.get(0)?,
                file_name: row.get(1)?,
                total_stamps: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        for row in rows {
            result.push(row?);
        }
    }
    
    Ok(result)
}

pub fn get_stamps_for_import(conn: &Connection, parent_hash: &str, available_only: bool, used_only: bool) -> Result<Vec<StampInfo>> {
    let query = if available_only {
        "SELECT id, stamp_index, matrix_number, printed_at FROM stamps WHERE parent_hash = ?1 AND printed_at IS NULL ORDER BY stamp_index"
    } else if used_only {
        "SELECT id, stamp_index, matrix_number, printed_at FROM stamps WHERE parent_hash = ?1 AND printed_at IS NOT NULL ORDER BY stamp_index"
    } else {
        "SELECT id, stamp_index, matrix_number, printed_at FROM stamps WHERE parent_hash = ?1 ORDER BY stamp_index"
    };
    
    let mut stmt = conn.prepare(query)?;
    let rows = stmt.query_map(params![parent_hash], |row| {
        Ok(StampInfo {
            id: row.get(0)?,
            stamp_index: row.get(1)?,
            matrix_number: row.get(2)?,
            printed_at: row.get(3)?,
        })
    })?;
    
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}
