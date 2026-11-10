//TODO: needs ctrl+z functionality

use arboard::{Clipboard, ImageData};
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use std::borrow::Cow;

fn draw_line(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    from: (f32, f32),
    to: (f32, f32),
    color: u32,
    radius: i32,
) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let steps = dx.abs().max(dy.abs()).ceil() as usize;

    for step in 0..=steps {
        let t = if steps == 0 {
            0.0
        } else {
            step as f32 / steps as f32
        };
        let x = (from.0 + dx * t).round() as i32;
        let y = (from.1 + dy * t).round() as i32;

        for oy in -radius..=radius {
            for ox in -radius..=radius {
                let px = x + ox;
                let py = y + oy;

                if px >= 0 && py >= 0 && (px as usize) < width && (py as usize) < height {
                    buffer[py as usize * width + px as usize] = color;
                }
            }
        }
    }
}

pub fn run_drawing_window() {
    let (width, height) = (300usize, 300usize);
    let mut buffer = vec![0x00ff_ffffu32; width * height]; // white canvas

    let mut window = Window::new("Draw", width, height, WindowOptions::default()).unwrap();
    let mut clipboard = Clipboard::new().expect("Could not open clipboard");

    // let mut right_was_down = false;

    let mut last_pos: Option<(f32, f32)> = None;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.get_mouse_down(MouseButton::Left) {
            if let Some(pos) = window.get_mouse_pos(MouseMode::Clamp) {
                if let Some(previous) = last_pos {
                    draw_line(&mut buffer, width, height, previous, pos, 0x0000_0000, 5);
                } else {
                    draw_line(&mut buffer, width, height, pos, pos, 0x0000_0000, 5);
                }
                last_pos = Some(pos);
            }
        } else {
            last_pos = None;
        }

        window.update_with_buffer(&buffer, width, height).unwrap();
    }

    let rgba: Vec<u8> = buffer
        .iter()
        .flat_map(|&p| {
            [
                ((p >> 16) & 0xff) as u8,
                ((p >> 8) & 0xff) as u8,
                (p & 0xff) as u8,
                255,
            ]
        })
        .collect();
    match clipboard.set_image(ImageData {
        width,
        height,
        bytes: Cow::Owned(rgba),
    }) {
        Ok(()) => println!("Copied canvas image to clipboard"),
        Err(e) => eprintln!("Clipboard error: {e}"),
    }

    //     let right_is_down = window.get_mouse_down(MouseButton::Right);
    //     if right_is_down && !right_was_down {
    //         let rgba: Vec<u8> = buffer
    //             .iter()
    //             .flat_map(|&p| [
    //                 ((p >> 16) & 0xff) as u8,
    //                 ((p >> 8) & 0xff) as u8,
    //                 (p & 0xff) as u8,
    //                 255,
    //             ])
    //             .collect();
    //
    //         match clipboard.set_image(ImageData {
    //             width,
    //             height,
    //             bytes: Cow::Owned(rgba),
    //         }) {
    //             Ok(()) => println!("Copied canvas image to clipboard"),
    //             Err(e) => eprintln!("Clipboard error: {e}"),
    //         }
    //     }
    //     right_was_down = right_is_down;
    //
    //     window.update_with_buffer(&buffer, width, height).unwrap();
}
