use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "postamt")]
#[command(about = "Briefmarken-Manager CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Register PDF and extract stamps")]
    Add {
        path: String,
        #[arg(long)]
        r#move: bool,
    },
    
    #[command(about = "Show count of available stamps")]
    Status,
    
    #[command(about = "List available profiles")]
    Profiles,
    
    #[command(about = "List available printers")]
    Printers,
    
    #[command(about = "Mark stamp as available")]
    MarkAvailable { identifier: String },
    
    #[command(about = "Mark stamp as used without printing")]
    MarkUsed { identifier: String },
    
    #[command(about = "Print envelope with stamp")]
    Print {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        printer: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
}
