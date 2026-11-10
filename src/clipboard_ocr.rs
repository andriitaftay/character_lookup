use arboard::{Clipboard, ImageData};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use leptess::LepTess;
use std::io::Cursor;

pub fn clipboard_image_to_png(image: ImageData<'_>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let width = image.width as u32;
    let height = image.height as u32;

    let rgba: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, image.bytes.into_owned())
            .ok_or("clipboard image has invalid dimensions or pixel data")?;

    let dynamic_image = DynamicImage::ImageRgba8(rgba);

    let mut png_data = Cursor::new(Vec::new());
    dynamic_image.write_to(&mut png_data, ImageFormat::Png)?;

    Ok(png_data.into_inner())
}

pub fn extract_character() -> Result<String, Box<dyn std::error::Error>> {
    let mut clipboard = Clipboard::new()?;

    let clipboard_image = clipboard
        .get_image()
        .map_err(|_| "the clipboard does not currently contain an image")?;

    let png_data = clipboard_image_to_png(clipboard_image)?;

    let mut ocr = LepTess::new(None, "chi_sim")?;

    ocr.set_image_from_mem(&png_data)?;

    let text = ocr.get_utf8_text()?;

    let trimmed_text = text.trim();
    let set_text_result = clipboard.set_text(trimmed_text);

    if set_text_result.is_err() {
        println!("{:?}", set_text_result.err());
    } else {
        println!("\"{}\" is added to clipboard", trimmed_text);
    }

    Ok(trimmed_text.to_string())
}
