use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("session error: {0}")]
    SessionError(#[from] super::client::session::Error),

    #[error("HTTP client error: {0}")]
    ClientError(#[from] super::client::Error),

    #[error("TUI error: {0}")]
    TuiError(#[from] super::tui::Error),

    #[error("IO error encountered - {0}")]
    IOError(#[from] std::io::Error),

    #[error("Failed to open attachment - {0}")]
    OpenError(#[from] opener::OpenError),

    #[error("Application error - {0}")]
    AppError(&'static str),

    #[error("User error - {0}")]
    UserError(&'static str),
}
