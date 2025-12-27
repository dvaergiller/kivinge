use qrcode::{EcLevel, QrCode, Version};
use qrcode_braille::render::BraillePixel;
use tracing::info;

use super::Error;

pub fn encode(code_data: &String) -> Result<String, Error> {
    let code = QrCode::with_version(
        code_data,
        Version::Normal(11),
        EcLevel::H,
    )?;
    Ok(code.render::<BraillePixel>().build())
}
