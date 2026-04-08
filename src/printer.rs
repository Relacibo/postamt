use crate::config::Config;
use crate::error::{Error, Result};
use std::io::{self, Write};
use std::process::Command;

pub fn get_printers() -> Vec<String> {
    let output = Command::new("lpstat").arg("-e").output();
    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|s| s.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn list_printers(config: &Config) -> Result<()> {
    let printers = get_printers();

    if printers.is_empty() {
        println!("No printers found");
        return Ok(());
    }

    // Get system default
    let default_output = Command::new("lpstat").arg("-d").output().ok();
    let system_default = default_output.and_then(|o| {
        let s = String::from_utf8_lossy(&o.stdout);
        s.split(':').nth(1).map(|p| p.trim().to_string())
    });

    println!("Available printers:");
    for printer in &printers {
        let mut markers = Vec::new();
        if Some(printer.clone()) == system_default {
            markers.push("system default");
        }
        if Some(printer) == config.default_printer.as_ref() {
            markers.push("config default");
        }
        if markers.is_empty() {
            println!("  {}", printer);
        } else {
            println!("  {} ({})", printer, markers.join(", "));
        }
    }
    Ok(())
}

pub fn prompt_for_printer() -> Result<Option<String>> {
    let printers = get_printers();

    if printers.is_empty() {
        return Err(Error::Custom("No printers found".to_string()));
    }

    println!("No default printer configured.");
    println!("Select a printer:");
    for (i, printer) in printers.iter().enumerate() {
        println!("  {}: {}", i + 1, printer);
    }
    println!("  0: Cancel");
    println!();
    println!("Tip: Set a default with: postamt config default_printer <name>");
    println!(
        "     e.g.: postamt config default_printer {}",
        printers.first().unwrap_or(&"<printer>".to_string())
    );
    print!("\nEnter number (0 to cancel): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() || input == "0" {
        return Ok(None);
    }

    match input.parse::<usize>() {
        Ok(n) if n >= 1 && n <= printers.len() => Ok(Some(printers[n - 1].clone())),
        _ => {
            println!("Invalid selection");
            Ok(None)
        }
    }
}

pub fn resolve_printer(config: &Config, printer_arg: Option<&str>) -> Result<Option<String>> {
    // 1. CLI argument takes precedence
    if let Some(p) = printer_arg {
        return Ok(Some(p.to_string()));
    }

    // 2. Config default
    if let Some(default_printer) = &config.default_printer {
        // Verify printer exists
        let printers = get_printers();
        if printers.contains(default_printer) {
            return Ok(Some(default_printer.clone()));
        } else {
            eprintln!(
                "Warning: Configured printer '{}' not found",
                default_printer
            );
        }
    }

    // 3. Prompt user
    prompt_for_printer()
}

pub fn print_pdf(printer: &str, pdf_path: &str) -> Result<()> {
    let status = Command::new("lpr")
        .arg("-P")
        .arg(printer)
        .arg(pdf_path)
        .status()?;

    if !status.success() {
        return Err(Error::Custom(format!(
            "Failed to print to printer: {}",
            printer
        )));
    }

    Ok(())
}
