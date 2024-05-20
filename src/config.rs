use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::path::PathBuf;
use anyhow::Context;

pub struct Config {
    pub api_key: String,
    pub auth_token: String,
}


pub fn get_config_path() -> anyhow::Result<PathBuf> {
    let path = dirs::home_dir()
        .map(|path| path.join(".marksman_config.yml"))
        .context("Could not find home directory")?;

    // check if the file exists and create it if it doesn't
    if !path.exists() {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .context("Failed to create config file")?;

        let default_config = Config {
            api_key: String::new(),
            auth_token: String::new(),
        };
        write_config(&default_config, &path)?;
    }

    Ok(path)
}

pub fn read_config(path: &PathBuf) -> anyhow::Result<Config> {
    let content = fs::read_to_string(path).context("Failed to read config file")?;
    let mut config_map = HashMap::new();

    for line in content.lines() {
        let mut parts = line.splitn(2, ':');
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            config_map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    let api_key = config_map.get("api_key").unwrap_or(&String::new()).clone();
    let auth_token = config_map.get("auth_token").unwrap_or(&String::new()).clone();

    Ok(Config { api_key, auth_token })
}

pub fn write_config(config: &Config, path: &PathBuf) -> anyhow::Result<()> {
    let config_content = format!("api_key: {}\nauth_token: {}", config.api_key, config.auth_token);
    fs::write(path, config_content.as_bytes()).context("Failed to write to config file")
}