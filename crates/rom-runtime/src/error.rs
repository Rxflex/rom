use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Core(#[from] rom_core::RomError),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error("invalid runtime URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
