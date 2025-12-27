use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("HTTP request failed - {0:?}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON encode/decode failed - {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("base64 decode failed - {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("Random data generation failed - {0}")]
    RandError(#[from] rand::Error),

    #[error("QR code generation failed - {0}")]
    QRError(#[from] super::qr::Error),

    #[error("IO error encountered - {0}")]
    IOError(#[from] std::io::Error),

    #[error("Application error - {0}")]
    AppError(String),
}
