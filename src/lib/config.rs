use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    fs::{File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
};
use serde_json;
use log::{debug};

#[derive(Serialize, Deserialize, Clone)]
pub struct PiConfig {
    pub servers: Vec<PiServer>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PiServer {
    pub base_url: String,
    pub api_key: String,
}

pub enum FromPath {
    IOError(String),
    FileNotFound(String),
    SerError(String),
}

impl PiConfig {
    pub fn new() -> Self {
        return Self { servers: vec![] };
    }

    /// Return the deserialized config or a JSON error if the file doesn't
    /// exist
    pub fn from_path(path: &Path) -> Result<PiConfig, FromPath> {
        debug!("Attempting to load the config from path: {}", path.to_string_lossy());

        let  fp = match File::open(path) {
            Ok(fp) => fp,
            Err(e) => {
                return Err(FromPath::FileNotFound(
                    format!("Failed to open file: {}", e)));
            },
        };
        let ret: PiConfig = match serde_json::from_reader(fp) {
            Ok(p) => p,
            Err(e) => {
                return Err(FromPath::SerError(
                    format!("Failed to deserialize the file: {}", e)));
            }
        };

        return Ok(ret);
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), FromPath> {
        debug!("Attempting to save the config to: {}", path.to_string_lossy());
        let fp = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path) {
                Ok(fp) => fp,
                Err(e) => {
                    return Err(FromPath::IOError(
                        format!("Failed to open {} for writing: {}",
                            path.display(), e)));
                }
            };
        if let Err(e) = serde_json::to_writer_pretty(fp, &self) {
            return Err(FromPath::SerError(
                format!("Failed to serialize the config: {}", e)));
        }

        return Ok(());
    }

    /// Add a server to the list
    pub fn add_server(&mut self, server: PiServer) {
        self.servers.push(server);
    }

}

impl PiServer {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        let base = base_url.trim_matches('/');

        return Self {
            base_url: base.to_string(),
            api_key: api_key.to_string(),
        };
    }
}