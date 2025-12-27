use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("HTTP request failed")]
    HttpError(reqwest::Error),
    #[error("JSON encode/decode failed")]
    JsonError(serde_json::Error),
    #[error("Random data generation failed")]
    RandError(rand::Error),
    #[error("QR code generation failed")]
    QRError(super::qr::Error),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error { Error::HttpError(e) }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error { Error::JsonError(e) }
}

impl From<rand::Error> for Error {
    fn from(e: rand::Error) -> Error { Error::RandError(e) }
}

impl From<super::qr::Error> for Error {
    fn from(e: super::qr::Error) -> Error { Error::QRError(e) }
}
