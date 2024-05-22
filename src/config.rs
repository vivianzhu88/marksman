use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Serialize, Deserialize};
use toml;
use chrono::{Utc, Duration, Date};


#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    pub api_key: String,
    pub auth_token: String,
    pub venue_id: String,
    pub date: String,
    pub party_size: u8,
}

impl Default for Config {
    fn default() -> Self {
        let one_week_later = Utc::now().date_naive() + Duration::days(7);
        Config {
            api_key: String::new(),
            auth_token: String::new(),
            venue_id: String::new(),
            date: one_week_later.format("%Y-%m-%d").to_string(),
            party_size: 2,
        }
    }
}

pub fn reset(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path).context("Failed to delete config file")?;
    }
    init_config(path)
}

fn init_config(path: &Path) -> Result<()> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .context("Failed to create config file")?;

    let default_config = Config::default();
    write_config(&default_config, Some(path))
}

pub fn get_config_path() -> Result<PathBuf> {
    let path = dirs::home_dir()
        .map(|path| path.join(".marksman.config"))
        .context("Could not find home directory")?;

    if !path.exists() {
        reset(&path)?;
    }

    Ok(path)
}

pub fn read_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path).context("Failed to read config file")?;
    let config: Config = toml::from_str(&content).context("Failed to deserialize config")?;
    Ok(config)
}

pub fn write_config(config: &Config, path: Option<&Path>) -> Result<()> {
    let config_path = path.cloned().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|home| home.join(".marksman.config"))
            .expect("Unable to determine home directory")
    });

    let config_content = toml::to_string(config).context("Failed to serialize config")?;
    fs::write(config_path, config_content.as_bytes())
        .context("Failed to write to config file")?;
    Ok(())
}
