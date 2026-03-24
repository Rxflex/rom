use thiserror::Error;

#[derive(Debug, Error)]
pub enum RomError {
    #[error("quickjs exception: {0}")]
    QuickJsException(String),
    #[error("quickjs error: {0}")]
    QuickJs(#[from] rquickjs::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, RomError>;
