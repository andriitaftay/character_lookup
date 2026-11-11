use ashpd::Error::*;
use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;
use std::error::Error;
mod clipboard_ocr;
use clipboard_ocr::*;

mod parse_config;

mod draw_char;
use draw_char::run_drawing_window;

mod lookup;
use lookup::run_lookup_window;

enum MyFunctions {
    Extract,
    Draw,
    Lookup,
}

async fn create_shortcuts() -> Result<Vec<NewShortcut>, Box<dyn Error>> {
    let shortcuts = parse_config::get_bindings()?;
    return Ok(shortcuts);
    // return Ok(shortcuts
    //     .into_iter()
    //     .map(|x| x.0.preferred_trigger(x.1.to_string()))
    //     .collect())
}

async fn init_extractor_portal(shortcuts: Vec<NewShortcut>) -> GlobalShortcuts {
    let portal = GlobalShortcuts::new().await.unwrap();

    let session = portal.create_session(Default::default()).await.unwrap();
    //
    //     let mut shortcut = NewShortcut::new(
    //         "extract_character",
    //         "Uses tesseract on a character in clipboard and copies character into it",
    //     );
    //     shortcut = shortcut.preferred_trigger(HOTKEY_EXTRACT);
    //
    let _ = portal
        .bind_shortcuts(&session, &shortcuts, None, BindShortcutsOptions::default())
        .await
        .unwrap()
        .response();

    return portal;
}

async fn actually_listen_for_hotkey(portal: &mut GlobalShortcuts) -> ashpd::Result<MyFunctions> {
    let mut activated = portal.receive_activated().await?;

    while let Some(event) = activated.next().await {
        if event.shortcut_id() == "extract_character" {
            return Ok(MyFunctions::Extract);
        }
        if event.shortcut_id() == "open_window_for_drawing_characters" {
            return Ok(MyFunctions::Draw);
        }
        if event.shortcut_id() == "lookup_character" {
            return Ok(MyFunctions::Lookup);
        }
    }

    Err(NoResponse)
}

async fn extract_character_on_blocking_thread(mode: OcrMode) -> Option<String> {
    // `extract_character` uses blocking I/O (clipboard, model download, ONNX
    // inference). In particular, `rapidocr-core`'s blocking HTTP downloader
    // creates and drops its own Tokio runtime, which panics if dropped on an
    // async runtime thread. Run it on a blocking worker instead.
    let result = tokio::task::spawn_blocking(move || {
        extract_character(mode).map_err(|error| error.to_string())
    })
    .await;

    match result {
        Ok(Ok(text)) => Some(text),
        Ok(Err(error)) => {
            println!("Failed to extract character: {error}");
            None
        }
        Err(error) => {
            println!("Character extraction task failed: {error}");
            None
        }
    }
}

async fn lookup_clipboard_character() {
    let result = tokio::task::spawn_blocking(|| {
        character_from_clipboard().map_err(|error| error.to_string())
    })
    .await;

    match result {
        Ok(Ok(character)) => {
            if let Err(error) = run_lookup_window(&character) {
                println!("Failed to open lookup window: {error}");
            }
        }
        Ok(Err(error)) => println!("Failed to read character from clipboard: {error}"),
        Err(error) => println!("Character lookup task failed: {error}"),
    }
}

fn open_lookup_window(character: &str) {
    if character.trim().is_empty() {
        return;
    }
    if let Err(error) = run_lookup_window(character) {
        println!("Failed to open lookup window: {error}");
    }
}

#[tokio::main]
async fn main() {
    match create_shortcuts().await {
        Ok(shortcuts) => {
            let mut portal = init_extractor_portal(shortcuts).await;

            loop {
                match actually_listen_for_hotkey(&mut portal).await {
                    Ok(MyFunctions::Extract) => {
                        let _ = extract_character_on_blocking_thread(OcrMode::DetectText).await;
                    }
                    Ok(MyFunctions::Draw) => {
                        let _ = run_drawing_window();
                        let character =
                            extract_character_on_blocking_thread(OcrMode::SingleCharacter).await;
                        if parse_config::open_lookup_after_drawing() {
                            if let Some(character) = character {
                                open_lookup_window(&character);
                            }
                        }
                    }
                    Ok(MyFunctions::Lookup) => {
                        lookup_clipboard_character().await;
                    }
                    Err(err) => {
                        println!("{:?}", err);
                    }
                }
            }
        }
        Err(error) => {
            println!("Failed to load shortcuts: {error}")
        }
    }
}

// OLD FUNCTIONS (I WILL DELETE THEM COMPLETELY LATER)
// async fn listen_for_hotkey() -> ashpd::Result<()> {
//     let portal = GlobalShortcuts::new().await?;
//
//     let session = portal.create_session(Default::default()).await?;
//
//     let mut shortcut = NewShortcut::new(
//         "extract_character",
//         "Uses tesseract on a character in clipboard and copies character into it",
//     );
//     shortcut = shortcut.preferred_trigger("CTRL+ALT+C");
//
//     let request = portal
//         .bind_shortcuts(&session, &[shortcut], None, BindShortcutsOptions::default())
//         .await?;
//
//     let _ = request.response()?;
//
//     let mut activated = portal.receive_activated().await?;
//
//     while let Some(event) = activated.next().await {
//         if event.shortcut_id() == "extract_character" {
//             return Ok(());
//         }
//     }
//
//     Err(NoResponse)
// }
