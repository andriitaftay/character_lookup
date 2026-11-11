use std::{
    error::Error,
    sync::{Mutex, OnceLock},
};

use arboard::{Clipboard, ImageData};
use image::{DynamicImage, ImageBuffer, RgbImage, Rgba};
use rapidocr_core::{
    RapidOcr,
    config::PipelineConfig,
    model::{ModelCache, ModelDownloadMode, model_set_by_name},
    types::OcrOutput,
};

/// RapidOCR model set used for Chinese text recognition.
const MODEL_SET_NAME: &str = "ppocrv5-ch-mobile";
/// Directory used to cache the downloaded ONNX models.
const DEFAULT_MODEL_DIR: &str = "models";

/// Which OCR pipeline to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OcrMode {
    /// Detect text lines first. Suited to screenshots that may contain several
    /// characters or lines of text.
    DetectText,
    /// Treat the whole image as a single character. Suited to the drawing
    /// canvas, where the detector often misses one large isolated glyph.
    SingleCharacter,
}

/// Building the pipeline loads ONNX sessions, which is expensive, so keep one
/// instance per pipeline alive for the lifetime of the process.
static DETECT_ENGINE: OnceLock<Mutex<RapidOcr>> = OnceLock::new();
static SINGLE_CHARACTER_ENGINE: OnceLock<Mutex<RapidOcr>> = OnceLock::new();

pub fn clipboard_image_to_rgb(image: ImageData<'_>) -> Result<RgbImage, Box<dyn Error>> {
    let width = image.width as u32;
    let height = image.height as u32;

    let rgba: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, image.bytes.into_owned())
            .ok_or("clipboard image has invalid dimensions or pixel data")?;

    Ok(DynamicImage::ImageRgba8(rgba).to_rgb8())
}

fn ocr_engine(
    slot: &'static OnceLock<Mutex<RapidOcr>>,
    pipeline: PipelineConfig,
) -> Result<&'static Mutex<RapidOcr>, Box<dyn Error>> {
    if let Some(engine) = slot.get() {
        return Ok(engine);
    }

    let model_set = model_set_by_name(MODEL_SET_NAME)
        .ok_or_else(|| format!("unknown RapidOCR model set {MODEL_SET_NAME:?}"))?;
    let model_dir =
        std::env::var("RAPIDOCR_MODEL_DIR").unwrap_or_else(|_| DEFAULT_MODEL_DIR.to_string());
    let cache = ModelCache::new(model_dir);

    cache
        .ensure_model_set_for_pipeline(model_set, pipeline, ModelDownloadMode::Missing)
        .map_err(|error| format!("failed to prepare RapidOCR models: {error:#}"))?;

    let config = cache.config_for(model_set).with_pipeline(pipeline);
    let engine = RapidOcr::new(config)
        .map_err(|error| format!("failed to initialize RapidOCR: {error:#}"))?;

    // Another caller may have initialized the engine while this one was
    // downloading/preparing the models, so ignore the "already set" case.
    let _ = slot.set(Mutex::new(engine));
    let engine = slot
        .get()
        .ok_or_else(|| "RapidOCR engine initialization failed".to_string())?;
    Ok(engine)
}

fn run_ocr(engine: &Mutex<RapidOcr>, image: &RgbImage) -> Result<OcrOutput, Box<dyn Error>> {
    let mut ocr = engine
        .lock()
        .map_err(|_| "RapidOCR engine lock was poisoned")?;
    ocr.run_image(image)
        .map_err(|error| format!("RapidOCR failed: {error:#}").into())
}

fn is_empty(output: &OcrOutput) -> bool {
    output.lines.iter().all(|line| line.text.trim().is_empty())
}

fn run_pipeline(mode: OcrMode, image: &RgbImage) -> Result<OcrOutput, Box<dyn Error>> {
    match mode {
        OcrMode::SingleCharacter => {
            let engine = ocr_engine(&SINGLE_CHARACTER_ENGINE, PipelineConfig::recognition_only())?;
            run_ocr(engine, image)
        }
        OcrMode::DetectText => {
            let engine = ocr_engine(&DETECT_ENGINE, PipelineConfig::full())?;
            let output = run_ocr(engine, image)?;

            if is_empty(&output) {
                // A single character filling the frame is frequently missed by
                // the text detector, so retry by recognizing the whole image.
                let engine =
                    ocr_engine(&SINGLE_CHARACTER_ENGINE, PipelineConfig::recognition_only())?;
                run_ocr(engine, image)
            } else {
                Ok(output)
            }
        }
    }
}

fn output_text(output: &OcrOutput) -> String {
    output
        .lines
        .iter()
        .map(|line| line.text.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn recognize_image(mode: OcrMode, image: &RgbImage) -> Result<String, Box<dyn Error>> {
    Ok(output_text(&run_pipeline(mode, image)?))
}

fn clipboard_rgb_image(clipboard: &mut Clipboard) -> Result<RgbImage, Box<dyn Error>> {
    let clipboard_image = clipboard
        .get_image()
        .map_err(|_| "the clipboard does not currently contain an image")?;
    clipboard_image_to_rgb(clipboard_image)
}

pub fn extract_character(mode: OcrMode) -> Result<String, Box<dyn Error>> {
    let mut clipboard = Clipboard::new()?;
    let rgb_image = clipboard_rgb_image(&mut clipboard)?;
    let text = recognize_image(mode, &rgb_image)?;

    let trimmed_text = text.trim();
    let set_text_result = clipboard.set_text(trimmed_text);

    if set_text_result.is_err() {
        println!("{:?}", set_text_result.err());
    } else {
        println!("\"{}\" is added to clipboard", trimmed_text);
    }

    Ok(trimmed_text.to_string())
}

/// Returns a character (or short word) from the clipboard, preferring text and
/// falling back to OCR of a clipboard image.
pub fn character_from_clipboard() -> Result<String, Box<dyn Error>> {
    let mut clipboard = Clipboard::new()?;

    if let Ok(text) = clipboard.get_text() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let clipboard_image = clipboard
        .get_image()
        .map_err(|_| "the clipboard does not currently contain text or an image")?;
    let rgb_image = clipboard_image_to_rgb(clipboard_image)?;

    recognize_image(OcrMode::SingleCharacter, &rgb_image)
}
