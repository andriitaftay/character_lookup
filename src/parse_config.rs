use ashpd::desktop::global_shortcuts::NewShortcut;
// BindShortcuts, BindShortcutsOptions, GlobalShortcuts,
use serde::Deserialize;
use std::{error::Error, fs};

#[derive(Debug, Deserialize)]
struct Config {
    bindings: Vec<Binding>,
    #[serde(default)]
    settings: Settings,
}

#[derive(Debug, Default, Deserialize)]
struct Settings {
    #[serde(default)]
    open_lookup_after_drawing: bool,
}

#[derive(Debug, Deserialize)]
struct Binding {
    name: String,
    description: String,
    hotkey: String,
}

fn read_config() -> Result<Config, Box<dyn Error>> {
    let text = fs::read_to_string("config.toml")?;
    Ok(toml::from_str(&text)?)
}

pub fn get_bindings() -> Result<Vec<NewShortcut>, Box<dyn Error>> {
    let config = read_config()?;

    for binding in &config.bindings {
        println!(
            "{}: {} ({})",
            binding.name, binding.description, binding.hotkey
        );
    }

    let shortcuts: Vec<NewShortcut> = config
        .bindings
        .into_iter()
        .map(|x| NewShortcut::new(x.name, x.description).preferred_trigger(x.hotkey.as_str()))
        .collect();

    Ok(shortcuts)
}

pub fn open_lookup_after_drawing() -> bool {
    read_config()
        .map(|config| config.settings.open_lookup_after_drawing)
        .unwrap_or(false)
}
