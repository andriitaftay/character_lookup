use std::error::Error;

use ab_glyph::{Font, FontVec, GlyphId, PxScale, ScaleFont};
use fontdb::{Database, Weight};
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 900;
const HEIGHT: usize = 450;
const LEFT_WIDTH: usize = 300;
const MARGIN: i32 = 20;

const BACKGROUND: Rgb<u8> = Rgb([255, 255, 255]);
const SEPARATOR: Rgb<u8> = Rgb([210, 210, 210]);
const TEXT: Rgb<u8> = Rgb([24, 24, 24]);
const MUTED_TEXT: Rgb<u8> = Rgb([120, 120, 120]);
const ENTRY_DIVIDER: Rgb<u8> = Rgb([226, 226, 226]);
const PINYIN_BOX_FILL: Rgb<u8> = Rgb([235, 242, 252]);
const PINYIN_BOX_BORDER: Rgb<u8> = Rgb([176, 198, 224]);
const PINYIN_TEXT: Rgb<u8> = Rgb([32, 68, 128]);

const PINYIN_PADDING_X: i32 = 8;
const PINYIN_PADDING_Y: i32 = 4;
const ENTRY_GAP: i32 = 12;
const DEFINITION_SEPARATOR: &str = " - ";

struct ChosenFont {
    font: FontVec,
    has_tone_marks: bool,
}

struct LookupEntry {
    pinyin: String,
    definitions: String,
}

/// Looks up `character` and shows a window with the character on the left and
/// its pinyin/definitions on the right.
pub fn run_lookup_window(character: &str) -> Result<(), Box<dyn Error>> {
    let character = character
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    if character.is_empty() {
        return Err("no character to look up".into());
    }

    let chosen = load_cjk_font()?;
    let entries = lookup_entries(character, chosen.has_tone_marks);
    let image = render_lookup_image(character, &entries, &chosen.font);
    show_window(&image, character)
}

fn load_cjk_font() -> Result<ChosenFont, String> {
    let mut db = Database::new();
    db.load_system_fonts();

    let mut best: Option<(i32, i32, FontVec, bool)> = None;

    for face in db.faces() {
        let score = face_score(face);
        if score == 0
            && best
                .as_ref()
                .is_some_and(|(best_score, _, _, _)| *best_score >= 2)
        {
            continue;
        }

        let Some(font) = db
            .with_face_data(face.id, |data, index| {
                FontVec::try_from_vec_and_index(data.to_vec(), index).ok()
            })
            .flatten()
        else {
            continue;
        };

        if font.glyph_id('中') == GlyphId(0) {
            continue;
        }

        let has_tone_marks = font.glyph_id('ǐ') != GlyphId(0);
        // Prefer a regular-weight face: exactly 400 is ideal, and a weight
        // close to 400 beats a bold face from the same family.
        let weight_penalty = (face.weight.0 as i32 - Weight::NORMAL.0 as i32).abs();
        let replace = best
            .as_ref()
            .map_or(true, |(best_score, best_penalty, _, _)| {
                score > *best_score || (score == *best_score && weight_penalty < *best_penalty)
            });
        if replace {
            best = Some((score, weight_penalty, font, has_tone_marks));
        }

        if score >= 3 && weight_penalty == 0 && has_tone_marks {
            break;
        }
    }

    best.map(|(_, _, font, has_tone_marks)| ChosenFont {
        font,
        has_tone_marks,
    })
    .ok_or_else(|| "no CJK-capable system font was found".to_string())
}

fn face_score(face: &fontdb::FaceInfo) -> i32 {
    let mut best = 0;
    for (family, _) in &face.families {
        let name = family.to_lowercase();
        let score = if name.contains("noto sans cjk sc")
            || name.contains("noto sans sc")
            || name.contains("source han sans sc")
            || name.contains("wenquanyi")
            || name.contains("droid sans fallback")
            || name.contains("simsun")
            || name.contains("microsoft yahei")
        {
            3
        } else if name.contains("cjk") || name.contains("han") {
            2
        } else {
            0
        };
        best = best.max(score);
    }
    best
}

fn lookup_entries(character: &str, has_tone_marks: bool) -> Vec<LookupEntry> {
    let mut matches = chinese_dictionary::query_by_simplified(character);
    if matches.is_empty() {
        matches = chinese_dictionary::query_by_traditional(character);
    }
    if matches.is_empty() {
        matches = chinese_dictionary::query(character).unwrap_or_default();
    }

    matches
        .into_iter()
        .take(3)
        .map(|entry| {
            let pinyin = if has_tone_marks && !entry.pinyin_marks.trim().is_empty() {
                entry.pinyin_marks.clone()
            } else if !entry.pinyin_numbers.trim().is_empty() {
                entry.pinyin_numbers.clone()
            } else {
                entry.pinyin_marks.clone()
            };

            let definitions = if entry.english.is_empty() {
                "—".to_string()
            } else {
                entry.english.join("; ")
            };

            LookupEntry {
                pinyin,
                definitions,
            }
        })
        .collect()
}

fn render_lookup_image(character: &str, entries: &[LookupEntry], font: &FontVec) -> RgbImage {
    let mut canvas = RgbImage::from_pixel(WIDTH as u32, HEIGHT as u32, BACKGROUND);

    for y in 0..HEIGHT as u32 {
        canvas.put_pixel(LEFT_WIDTH as u32, y, SEPARATOR);
    }

    draw_character(&mut canvas, character, font);
    draw_entries(&mut canvas, entries, font);

    canvas
}

fn draw_character(canvas: &mut RgbImage, text: &str, font: &FontVec) {
    if text.is_empty() {
        return;
    }

    let max_width = LEFT_WIDTH as f32 - 2.0 * MARGIN as f32;
    let max_height = HEIGHT as f32 - 2.0 * MARGIN as f32;

    let mut scale = 240.0f32;
    loop {
        let px_scale = PxScale::from(scale);
        let (width, _) = text_size(px_scale, font, text);
        let scaled = font.as_scaled(px_scale);
        let line_height = scaled.ascent() - scaled.descent();

        if (width as f32 <= max_width && line_height <= max_height) || scale <= 12.0 {
            break;
        }
        scale *= 0.9;
    }

    let px_scale = PxScale::from(scale);
    let (width, _) = text_size(px_scale, font, text);
    let scaled = font.as_scaled(px_scale);
    let line_height = scaled.ascent() - scaled.descent();
    let x = ((LEFT_WIDTH as f32 - width as f32) / 2.0).max(0.0) as i32;
    let y = ((HEIGHT as f32 - line_height) / 2.0).max(0.0) as i32;

    draw_text_mut(canvas, TEXT, x, y, px_scale, font, text);
}

fn draw_entries(canvas: &mut RgbImage, entries: &[LookupEntry], font: &FontVec) {
    let x0 = LEFT_WIDTH as i32 + MARGIN;
    let right_edge = WIDTH as i32 - MARGIN;
    let scale = PxScale::from(22.0);
    let scaled = font.as_scaled(scale);
    let line_height = (scaled.ascent() - scaled.descent()).ceil() as i32;
    let line_step = (line_height as f32 * 1.25).round() as i32;
    let max_y = HEIGHT as i32 - MARGIN;
    let mut y = MARGIN;

    if entries.is_empty() {
        draw_text_mut(
            canvas,
            MUTED_TEXT,
            x0,
            y,
            scale,
            font,
            "No dictionary entry found.",
        );
        return;
    }

    for (index, entry) in entries.iter().enumerate() {
        let (pinyin_width, _) = text_size(scale, font, &entry.pinyin);
        let box_width = pinyin_width as i32 + PINYIN_PADDING_X * 2;
        let box_height = line_height + PINYIN_PADDING_Y * 2;

        if y + box_height > max_y {
            break;
        }

        let box_rect = Rect::at(x0, y).of_size(box_width as u32, box_height as u32);
        draw_filled_rect_mut(canvas, box_rect, PINYIN_BOX_FILL);
        draw_hollow_rect_mut(canvas, box_rect, PINYIN_BOX_BORDER);

        let text_y = y + PINYIN_PADDING_Y;
        draw_text_mut(
            canvas,
            PINYIN_TEXT,
            x0 + PINYIN_PADDING_X,
            text_y,
            scale,
            font,
            &entry.pinyin,
        );

        let (separator_width, _) = text_size(scale, font, DEFINITION_SEPARATOR);
        let separator_x = x0 + box_width;
        draw_text_mut(
            canvas,
            MUTED_TEXT,
            separator_x,
            text_y,
            scale,
            font,
            DEFINITION_SEPARATOR,
        );

        let def_x = separator_x + separator_width as i32;
        let first_width = (right_edge - def_x).max(0) as u32;
        let next_width = (right_edge - x0).max(0) as u32;
        let lines = wrap_text_hanging(font, scale, &entry.definitions, first_width, next_width);

        let mut line_y = text_y;
        for (line_index, line) in lines.iter().enumerate() {
            let line_x = if line_index == 0 { def_x } else { x0 };
            if line_y + line_height > max_y {
                break;
            }
            draw_text_mut(canvas, TEXT, line_x, line_y, scale, font, line);
            line_y += line_step;
        }

        let definitions_bottom = if lines.is_empty() {
            text_y + line_height
        } else {
            text_y + (lines.len() as i32 - 1) * line_step + line_height
        };
        let entry_bottom = (y + box_height).max(definitions_bottom);
        y = entry_bottom + ENTRY_GAP;

        if index + 1 < entries.len() && y < max_y {
            draw_filled_rect_mut(
                canvas,
                Rect::at(x0, y).of_size((right_edge - x0) as u32, 1),
                ENTRY_DIVIDER,
            );
            y += ENTRY_GAP;
        }
    }
}

fn wrap_text_hanging(
    font: &FontVec,
    scale: PxScale,
    text: &str,
    first_width: u32,
    next_width: u32,
) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width_limit = first_width;

    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };

        if current.is_empty() || text_size(scale, font, &candidate).0 <= width_limit {
            current = candidate;
        } else {
            lines.push(current);
            current = word.to_string();
            width_limit = next_width;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn show_window(image: &RgbImage, character: &str) -> Result<(), Box<dyn Error>> {
    let buffer: Vec<u32> = image
        .pixels()
        .map(|pixel| ((pixel[0] as u32) << 16) | ((pixel[1] as u32) << 8) | pixel[2] as u32)
        .collect();

    let title = format!("Lookup: {character}");
    let mut window = Window::new(
        &title,
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(30);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update_with_buffer(&buffer, WIDTH, HEIGHT)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_common_characters() {
        let entries = lookup_entries("中", true);
        assert!(!entries.is_empty());
        assert!(
            entries
                .iter()
                .any(|entry| entry.pinyin.to_lowercase().contains("zh"))
        );
    }

    #[test]
    fn render_preview() {
        let chosen = load_cjk_font().unwrap();
        for character in ["中", "行"] {
            let entries = lookup_entries(character, chosen.has_tone_marks);
            let image = render_lookup_image(character, &entries, &chosen.font);
            image
                .save(format!("target/lookup_styled_{character}.png"))
                .unwrap();
        }
    }
}
