use super::Error;
use qrcode2::{EcLevel, QrCode, Version};
use qrcode_unicode_ext::BraillePixel;

pub fn encode(code_data: &String) -> Result<String, Error> {
    let code =
        QrCode::with_version(code_data, Version::Normal(11), EcLevel::H)?;
    Ok(code
        .render::<BraillePixel>()
        .dark_color(BraillePixel::Light)
        .light_color(BraillePixel::Dark)
        .build())
}
