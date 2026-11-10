use ashpd::desktop::global_shortcuts::NewShortcut;
// BindShortcuts, BindShortcutsOptions, GlobalShortcuts,
use serde::Deserialize;
use std::{error::Error, fs};

#[derive(Debug, Deserialize)]
struct Config {
    bindings: Vec<Binding>,
}

#[derive(Debug, Deserialize)]
struct Binding {
    name: String,
    description: String,
    hotkey: String,
}

pub fn get_bindings() -> Result<Vec<NewShortcut>, Box<dyn Error>> {
    let text = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&text)?;

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
