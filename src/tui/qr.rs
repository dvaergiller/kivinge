use qrcode::render::unicode::Dense1x2;
use qrcode::QrCode;

use super::Error;

pub fn encode(code_data: &String) -> Result<String, Error> {
    let result = QrCode::new(code_data)?.render::<Dense1x2>().build();
    Ok(result)
}
