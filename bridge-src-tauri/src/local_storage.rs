use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use pluralsync_base::for_discord_bridge;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, fs};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub base_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: env::var("PLURALSYNC_BASE_URL")
                .unwrap_or_else(|_| for_discord_bridge::CANONICAL_PLURALSYNC_BASE_URL.to_owned()),
        }
    }
}

fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = get_data_dir()?;
    fs::create_dir_all(&proj_dirs)?;
    Ok(proj_dirs.join("config.json"))
}

fn get_config() -> Result<Config> {
    let path = get_config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let json = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&json)?;
    Ok(config)
}

fn set_config(config: &Config) -> Result<()> {
    let path = get_config_path()?;
    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn set_base_url(base_url: String) -> Result<()> {
    let mut config = get_config()?;
    config.base_url = base_url;
    set_config(&config)?;
    Ok(())
}

pub fn get_base_url() -> Result<String> {
    get_config().map(|c| c.base_url)
}

pub fn get_logs_dir() -> Result<PathBuf> {
    Ok(get_data_dir()?.join("logs"))
}

fn get_data_dir() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("io", "pluralsync", "pluralsync.bridge")
        .ok_or_else(|| anyhow!("get_data_dir: Failed to get project directories"))
        .map(|p| p.data_local_dir().to_path_buf());

    let data_dir = env::var("PLURALSYNC_DATA_DIR")
        .map(PathBuf::from)
        .or(proj_dirs)?;

    log::debug!("Data dir: {}", data_dir.to_string_lossy());

    Ok(data_dir)
}

fn get_credentials_path() -> Result<PathBuf> {
    let data_dir = get_data_dir()?;
    fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join("credentials.json"))
}

pub fn get_user_credentials() -> Result<for_discord_bridge::UserLoginCredentials> {
    let path = get_credentials_path()?;
    let json = fs::read_to_string(path)?;
    let creds: for_discord_bridge::UserLoginCredentials = serde_json::from_str(&json)?;
    log::info!("Retrieved credentials for {}", &creds.email);
    Ok(creds)
}

pub fn set_user_credentials(creds: &for_discord_bridge::UserLoginCredentials) -> Result<()> {
    let path = get_credentials_path()?;
    let json = serde_json::to_string(creds)?;
    fs::write(path, json)?;
    log::info!("Stored credentials for {}", &creds.email);
    Ok(())
}

pub fn clear_user_credentials() -> Result<()> {
    let path = get_credentials_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    log::info!("Cleared credentials.");
    Ok(())
}
