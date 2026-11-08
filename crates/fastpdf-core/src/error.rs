use thiserror::Error;

#[derive(Debug, Error)]
pub enum FastPdfError {
    #[error("corrupt or invalid PDF: {0}")]
    CorruptPdf(String),
    #[error("invalid password")]
    InvalidPassword,
    #[error("page index {0} out of range")]
    PageOutOfRange(u32),
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("lopdf error: {0}")]
    Lopdf(String),
}

pub type Result<T> = std::result::Result<T, FastPdfError>;

impl From<lopdf::Error> for FastPdfError {
    fn from(e: lopdf::Error) -> Self {
        let msg = e.to_string();
        if msg.to_lowercase().contains("password") {
            FastPdfError::InvalidPassword
        } else {
            FastPdfError::Lopdf(msg)
        }
    }
}
