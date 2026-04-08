use crate::error::{Error, Result};
use crate::config::Config;
use std::process::Command;

pub fn list_printers(config: &Config) -> Result<()> {
    let output = Command::new("lpstat")
        .args(&["-p", "-d"])
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            
            if lines.is_empty() {
                println!("No printers found");
                return Ok(());
            }
            
            println!("Available printers:");
            for line in lines {
                if line.starts_with("printer ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 1 {
                        let printer_name = parts[1];
                        let marker = if printer_name == config.default_printer {
                            " (default)"
                        } else {
                            ""
                        };
                        println!("  {}{}", printer_name, marker);
                    }
                } else if line.starts_with("system default destination:") {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() > 1 {
                        let default_printer = parts[1].trim();
                        println!("System default: {}", default_printer);
                    }
                }
            }
            Ok(())
        }
        _ => {
            println!("No printers found");
            Ok(())
        }
    }
}

pub fn print_pdf(printer: &str, pdf_path: &str) -> Result<()> {
    let status = Command::new("lpr")
        .arg("-P")
        .arg(printer)
        .arg(pdf_path)
        .status()?;
    
    if !status.success() {
        return Err(Error::Custom(format!("Failed to print to printer: {}", printer)));
    }
    
    Ok(())
}
