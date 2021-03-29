use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use bevy::asset::FileAssetIo;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub start_map: PathBuf,
    pub start_dialogue: PathBuf,

    pub char_template: String,
    pub char_height: f32,
    pub char_width: f32,
}

pub fn load_asset_config(name: &str) -> Result<Config> {
    let mut asset_path = FileAssetIo::get_root_path();
    asset_path.push("assets");
    asset_path.push(name);

    let contents = fs::read_to_string(asset_path.as_path())
        .with_context(||
            format!("error reading config file: {:?}",
                    asset_path.as_os_str())
        )?;
    let config = toml::from_str(contents.as_ref())
        .with_context(||
            format!("error parsing config file to expected TOML format: {:?}",
                    asset_path.as_os_str())
        )?;

    Ok(config)
}
