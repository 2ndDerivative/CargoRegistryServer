use std::{net::{IpAddr, SocketAddr, Ipv4Addr}, path::PathBuf};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize, Serializer};
use url::{Url, ParseError};

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let args = std::env::args().collect::<Vec<_>>();
    let Some(arg_path) = args.get(1) else {
        println!("Using default configuration");
        return Config::default();
    };
    let arg_path = PathBuf::from(arg_path);
    println!("Reading config at {}", arg_path.display());
    let s = std::fs::read_to_string(arg_path).expect("Reading config file failed. Does it exist?");
    toml::from_str(&s).expect("Invalid configuration TOML")
});

#[derive(Debug, Deserialize, Clone, Default)]
#[allow(dead_code)]
pub struct Config {
    pub index: IndexConfig,
    pub download: DownloadConfig,
    pub net: NetConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct NetConfig {
    pub ip: IpAddr,
    pub port: u16,
    pub threads: Option<usize>,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 7878,
            threads: Some(1)
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct DownloadConfig {
    pub path: String,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            path: String::from("target/debug/download")
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("database")
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct IndexConfig {
    pub path: PathBuf,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("target/debug/index")
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IndexConfigFile {
    dl: Url,
    #[serde(serialize_with = "url_without_trailing_slash")]
    api: Url
}

impl TryFrom<Config> for IndexConfigFile {
    fn try_from(value: Config) -> Result<Self, ParseError> {
        let socket_addr = SocketAddr::new(value.net.ip, value.net.port);
        Ok(Self {
            dl: format!("http://{socket_addr}").parse::<Url>()?.join(&value.download.path)?,
            api: format!("http://{socket_addr}").parse()?
        })
    }

    type Error = ParseError;
}

fn url_without_trailing_slash<S: Serializer>(url: &Url, s: S) -> Result<S::Ok, S::Error>{
    let st = url.as_str();
    s.serialize_str(st.strip_suffix('/').unwrap_or(st))
}