use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Serialize, Deserialize};
use toml;
use chrono::{Utc, Duration};


#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    pub api_key: String,

    #[serde(default)]
    pub auth_token: String,

    #[serde(default)]
    pub venue_id: String,

    #[serde(default = "_default_date")]
    pub date: String,

    #[serde(default = "_default_party_size")]
    pub party_size: u8,

    pub target_time: Option<String>,

    #[serde(default)]
    pub payment_id: String
}

fn _default_date() -> String {
    let one_week_later = Utc::now().date_naive() + Duration::days(7);
    one_week_later.format("%Y-%m-%d").to_string()
}

const fn _default_party_size() -> u8 { 2 }

impl Default for Config {
    fn default() -> Self {
        let one_week_later = Utc::now().date_naive() + Duration::days(7);
        Config {
            api_key: String::new(),
            auth_token: String::new(),
            venue_id: String::new(),
            date: one_week_later.format("%Y-%m-%d").to_string(),
            party_size: 2,
            target_time: None,
            payment_id: String::new(),
        }
    }
}

pub fn reset(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).context("Failed to delete config file")?;
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
    let config_path = path.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        dirs::home_dir()
            .map(|home| home.join(".marksman/config")) // Corrected the path to use a subdirectory
            .expect("Unable to determine home directory")
    });

    let config_content = toml::to_string(config).context("Failed to serialize config")?;
    fs::write(config_path, config_content.as_bytes())
        .context("Failed to write to config file")?;
    Ok(())
}
