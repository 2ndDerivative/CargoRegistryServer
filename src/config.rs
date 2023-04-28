use std::{net::{IpAddr, SocketAddr}, path::PathBuf};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize, Serializer};
use url::{Url, ParseError};

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    println!("Reading config...");
    let s = std::fs::read_to_string("config.toml").expect("did not find config.toml");
    toml::from_str(&s).expect("Unable to parse config")
});

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub index: IndexConfig,
    pub download: DownloadConfig,
    pub net: NetConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NetConfig {
    pub ip: IpAddr,
    pub port: u16,
    pub threads: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DownloadConfig {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexConfig {
    pub path: PathBuf,
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
            dl: format!("http://{}", socket_addr).parse::<Url>()?.join(&value.download.path)?,
            api: format!("http://{}", socket_addr).parse()?
        })
    }

    type Error = ParseError;
}

fn url_without_trailing_slash<S: Serializer>(url: &Url, s: S) -> Result<S::Ok, S::Error>{
    let st = url.as_str();
    s.serialize_str(st.strip_suffix('/').unwrap_or(st))
}