use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("PDF error: {0}")]
    Pdf(#[from] lopdf::Error),

    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;
