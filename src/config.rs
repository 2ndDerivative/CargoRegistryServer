use std::{net::SocketAddr, path::PathBuf};

use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    println!("Reading config...");
    let s = std::fs::read_to_string("config.toml").expect("did not find config.toml");
    toml::from_str(&s).expect("Unable to parse config")
});

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Config {
    pub index: IndexConfig,
    pub download: DownloadConfig,
    pub net: NetConfig,
}

#[derive(Debug, Deserialize)]
pub struct NetConfig {
    pub ip: SocketAddr,
    pub threads: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct DownloadConfig {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct IndexConfig {
    pub path: PathBuf,
}