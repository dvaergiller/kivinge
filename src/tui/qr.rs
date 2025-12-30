use qrcode::{render::braille::BraillePixel, EcLevel, QrCode, Version};

use super::Error;

pub fn encode(code_data: &String) -> Result<String, Error> {
    let code =
        QrCode::with_version(code_data, Version::Normal(11), EcLevel::H)?;
    Ok(code
        .render::<BraillePixel>()
        .dark_color(BraillePixel::Light)
        .light_color(BraillePixel::Dark)
        .build())
}
