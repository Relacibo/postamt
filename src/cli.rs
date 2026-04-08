use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use postamt_rs::OutputFormat;

#[derive(Parser)]
#[command(name = "postamt")]
#[command(about = "Briefmarken-Manager CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormatArg {
    Toml,
    Json,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Toml => OutputFormat::Toml,
            OutputFormatArg::Json => OutputFormat::Json,
        }
    }
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
        #[arg(help = "Specific stamp identifier to print")]
        identifier: Option<String>,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        printer: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, help = "Force printing even if stamp is marked as used")]
        force: bool,
        #[arg(long, help = "Rotate envelope 90° counter-clockwise for portrait printing")]
        rotate: bool,
    },
    
    #[command(about = "List stamps and files")]
    List {
        #[arg(long, help = "Filter by file hash")]
        file: Option<String>,
        #[arg(long, help = "Show only available stamps")]
        available: bool,
        #[arg(long, help = "Show only used stamps")]
        used: bool,
        #[arg(long, value_enum, default_value = "toml")]
        format: OutputFormatArg,
    },
    
    #[command(about = "Get or set config values")]
    Config {
        #[arg(help = "Key to get/set (e.g. default_printer, profiles.DL.width)")]
        key: Option<String>,
        #[arg(help = "Value to set")]
        value: Option<String>,
    },

    #[command(about = "Generate shell completions")]
    Completions {
        #[arg(value_enum, help = "Shell to generate completions for")]
        shell: Shell,
    },
}
