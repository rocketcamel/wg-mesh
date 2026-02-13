use std::path::PathBuf;

use crate::error::{Error, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub interface: InterfaceConfig,
    pub server: ServerConfig,
}
#[derive(Deserialize)]
pub struct InterfaceConfig {
    pub private_key: String,
    pub listen_port: u16,
    pub address: String,
}
#[derive(Deserialize)]
pub struct ServerConfig {
    pub url: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = base_path().join("config.toml");
        let bytes = std::fs::read(&path).map_err(|e| Error::read_config(e, &path))?;
        Ok(toml::from_slice(&bytes)?)
    }
}

pub fn base_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("wg-mesh")
}

pub fn wg_config_path() -> PathBuf {
    PathBuf::from("/etc/wireguard/mesh0.conf")
}
