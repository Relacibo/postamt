pub mod config;
pub mod db;
pub mod error;
pub mod pdf;
pub mod printer;
pub mod stamp;

#[derive(Clone)]
pub enum OutputFormat {
    Toml,
    Json,
}
