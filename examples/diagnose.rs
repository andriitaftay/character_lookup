use rapidocr_core::{
    RapidOcr,
    config::{PipelineConfig, RapidOcrConfig},
    model::{ModelCache, ModelDownloadMode, model_set_by_name},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: diagnose <image>")?;
    let image = image::open(&path)?.to_rgb8();
    println!("image: {}x{}", image.width(), image.height());

    let model_set = model_set_by_name("ppocrv5-ch-mobile").ok_or("model set missing")?;
    let cache = ModelCache::new("models");
    cache.ensure_model_set_for_pipeline(
        model_set,
        PipelineConfig::full(),
        ModelDownloadMode::Never,
    )?;

    for (label, pipeline, text_score) in [
        ("full/0.5", PipelineConfig::full(), 0.5f32),
        ("full/0.0", PipelineConfig::full(), 0.0),
        ("rec-only/0.0", PipelineConfig::recognition_only(), 0.0),
        ("rec-only/0.5", PipelineConfig::recognition_only(), 0.5),
    ] {
        let mut config: RapidOcrConfig = cache.config_for(model_set).with_pipeline(pipeline);
        config.text_score = text_score;
        let mut ocr = RapidOcr::new(config)?;
        let output = ocr.run_image(&image)?;
        println!("--- {label}: {} lines", output.lines.len());
        for line in output.lines {
            println!(
                "  {:?} score={:.4} bbox={:?}",
                line.text, line.score, line.bbox.points
            );
        }
    }

    Ok(())
}
