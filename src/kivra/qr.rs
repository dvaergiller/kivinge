use qrcode::render::unicode::Dense1x2;
use qrcode::QrCode;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("QR Error: {qr_error}")]
pub struct Error {
    pub qr_error: Box<dyn std::error::Error>,
}

pub fn encode(code_data: &String) -> Result<String, super::error::Error> {
    let result = QrCode::new(code_data)
        .map_err(|e| Error { qr_error: e.into() })?
        .render::<Dense1x2>()
        .build();
    Ok(result)
}
