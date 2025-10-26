use std::path::Path;
use serde::Deserialize;
use serde::de::Deserializer;
use quick_xml::de::from_str;
use tracing::error;

const DEFAULT_HOST: &str = "127.0.0.1";

fn default_host() -> String {
    DEFAULT_HOST.to_string()
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub run_time: RunTime,
    pub servers: Servers,
    pub log: Log,
    pub author:Author,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Servers {
    pub group: Vec<ServerGroup>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerGroup {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(default, rename = "@front")]
    pub front: bool,
    pub server: Vec<ServerConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    #[serde(rename = "@id")]
    pub id: u32,
    #[serde(default = "default_host", rename = "@host")]
    pub host: String,
    #[serde(default = "default_host", rename = "@front_host")]
    pub front_host: String,
    #[serde(default = "default_host", rename = "@back_host")]
    pub back_host: String,
    #[serde(rename = "@back_tcp_port")]
    pub back_tcp_port: u16,
    #[serde(rename = "@front_tcp_port")]
    pub front_tcp_port: Option<u16>,
    #[serde(rename = "@front_ws_port")]
    pub front_ws_port: Option<u16>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Log {
    #[serde(rename = "@debug")]
    pub debug: String,
    #[serde(rename = "@info")]
    pub info: String,
    #[serde(rename = "@net")]
    pub net: String,
    #[serde(rename = "@warn")]
    pub warn: String,
    #[serde(rename = "@err")]
    pub err: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Author {
    #[serde(rename = "@key")]
    pub key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RunTime {
    #[serde(rename = "@worker_threads")]
    pub worker_threads: u32,
}

impl Config {
    pub fn find_server(&self, server_id: u32) -> Option<(&ServerConfig, &ServerGroup)> {
        for group in &self.servers.group {
            if let Some(server) = group.server.iter().find(|s| s.id == server_id) {
                return Some((server, group));
            }
        }
        None
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(format!("Config file does not exist: {}", path_ref.display()).into());
        }

        let config_content = std::fs::read_to_string(path_ref)?;
        let config = quick_xml::de::from_str(&config_content)?;
        Ok(config)
    }
}
