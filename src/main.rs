mod cli;

use clap::Parser;
use cli::{Cli, Commands};
use postamt_rs::*;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> error::Result<()> {
    let cli = Cli::parse();
    
    let config = config::load()?;
    let db_path = db::get_db_path()?;
    let conn = db::init(&db_path)?;

    match cli.command {
        Commands::Add { path, r#move } => {
            stamp::add_stamps(&conn, &config, &path, r#move)?;
        }
        Commands::Status => {
            stamp::show_status(&conn)?;
        }
        Commands::Profiles => {
            config::show_profiles(&config)?;
        }
        Commands::Printers => {
            printer::list_printers(&config)?;
        }
        Commands::MarkAvailable { identifier } => {
            stamp::mark_available(&conn, &identifier)?;
        }
        Commands::MarkUsed { identifier } => {
            stamp::mark_used(&conn, &identifier)?;
        }
        Commands::Print { profile, printer, dry_run } => {
            stamp::print_stamp(&conn, &config, profile.as_deref(), printer.as_deref(), dry_run)?;
        }
        Commands::List { file, available, used, format } => {
            stamp::list_stamps(&conn, file.as_deref(), available, used, format.into())?;
        }
        Commands::Config { key, value } => {
            config::handle_config_command(key.as_deref(), value.as_deref())?;
        }
    }

    Ok(())
}
