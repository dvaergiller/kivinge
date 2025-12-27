use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("session manager error: {0}")]
    SessionManagerError(#[from] super::client::session_manager::Error),

    #[error("session error: {0}")]
    SessionError(#[from] super::client::session::Error),

    #[error("HTTP client error: {0}")]
    ClientError(#[from] super::client::Error),

    #[error("TUI error: {0}")]
    TuiError(#[from] super::tui::Error),

    // #[error("JSON encode/decode failed - {0}")]
    // JsonError(#[from] serde_json::Error),

    // #[error("base64 decode failed - {0}")]
    // Base64Error(#[from] base64::DecodeError),

    // #[error("UTF-8 decode error")]
    // FromUtf8Error(#[from] string::FromUtf8Error),

    // #[error("Random data generation failed - {0}")]
    // RandError(#[from] rand::Error),

    // #[error("QR code generation failed - {0}")]
    // QRError(#[from] super::tui::qr::Error),

    #[error("IO error encountered - {0}")]
    IOError(#[from] std::io::Error),

    #[error("Failed to open attachment - {0}")]
    OpenError(#[from] opener::OpenError),

    #[error("Application error - {0}")]
    AppError(&'static str),

    #[error("User error - {0}")]
    UserError(&'static str),
}
