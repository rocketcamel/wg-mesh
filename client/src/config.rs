use std::path::PathBuf;

use crate::error::{Error, Result};
use ipnetwork::IpNetwork;
use serde::Deserialize;

pub const DEFAULT_KEEPALIVE: u16 = 25;
pub const INTERFACE_NAME: &str = "mesh0";

#[derive(Deserialize)]
pub struct Config {
    pub interface: InterfaceConfig,
    pub server: ServerConfig,
}

#[derive(Deserialize)]
pub struct InterfaceConfig {
    pub private_key: String,
    pub public_key: String,
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
    #[serde(default = "default_keepalive")]
    pub persistent_keepalive: u16,
    pub allowed_ips: Option<Vec<IpNetwork>>,
}

fn default_listen_port() -> u16 {
    51820
}
fn default_keepalive() -> u16 {
    DEFAULT_KEEPALIVE
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub ws_url: String,
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
