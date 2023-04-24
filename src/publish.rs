use std::{
    io::{Read, Write, Result as IoResult},
    net::TcpStream, 
    collections::HashMap, 
    path::PathBuf, 
    fs::{OpenOptions, File},
};
use crate::{
    index_crate::IndexCrate, 
    dependency::Dependency, 
    git::add_and_commit_to_index, 
    http::{Response, Byteable}, 
    error_json, 
    config::CONFIG,
};
use walkdir::WalkDir;
use serde::{Deserialize, Serialize, de::Error};

use self::error::{PublishError, ReadStreamError};

pub mod error;
type PublishResult<T> = core::result::Result<T, PublishError>;

pub(crate) fn handle_publish_request(mut stream: TcpStream) -> IoResult<()> {
    println!("Publish request recognized!");
    let (index_crate, raw_crate_file) = match get_crate_and_raw_bytes_from_stream(&mut stream) {
        Ok(t) => t,
        Err(e) => {
            use ReadStreamError::*;
            let code = match e {
                ConnectionClosed(e) => return Err(e),
                BadHTTPJson(_) | NonNumericContentLength(_) | InvalidUTF8Error(_) => 400,
                PayloadTooLarge => 413,
            };
            let response = Response::new(code).body(error_json(&[&format!("{e}")]));
            return stream.write_all(&response.into_bytes())
        }
    };
    
    match process_publish_request(&index_crate, &raw_crate_file) {
        Ok(_) => {
            let warnings_json = serde_json::to_string(
                &ReturnJson::new()).expect("This is a static json object");
            Ok(stream.write_all(&Response::new(200).body(warnings_json).into_bytes())?)
        },
        Err(pub_err) => {
            use PublishError::*;
            let code = match pub_err {
                IoError(e) => return Err(e),
                VersionAlreadyExists | CrateExistsWithDifferentDashUnderscore => 403,
                BadIndexJson | SerializationFailed(_) => 500,
            };
            let response = Response::new(code).body(error_json(&[&format!("{pub_err}")]));
            stream.write_all(&response.into_bytes())
        }
    }
}

fn process_publish_request(index_crate: &IndexCrate, raw_file_bytes: &[u8]) -> PublishResult<()> {
    // Check for existing version
    let index_file_path = index_crate.path();
    for any_crate_file in WalkDir::new(&CONFIG.index.path).into_iter().flatten().filter(|p| 
        p.path().is_file() 
        && !p.path().starts_with(CONFIG.index.path.join(".git")) 
        && p.path() != CONFIG.index.path.join("config.json")){
            if let Some(file_name) = any_crate_file.file_name().to_str() {
                if file_name.replace('-', "_") == index_crate.name.replace('-', "_") {
                    if file_name != index_crate.name {
                        return Err(PublishError::CrateExistsWithDifferentDashUnderscore)
                    } else {
                        let index_file_read = std::fs::read_to_string(any_crate_file.path())?;
                        for n in index_file_read.lines() {
                            let line_crate: IndexCrate = serde_json::from_str(n).map_err(|_| PublishError::BadIndexJson)?;
                            if line_crate.vers == index_crate.vers {
                                return Err(PublishError::VersionAlreadyExists)
                            } 
                        }
                    }
                }
            }

        };

    if !index_file_path.exists() {
        if let Some(parent) = index_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut index_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&index_file_path)?;

    write!(index_file, "{}\r\n", serde_json::to_string(&index_crate)?)?;
    drop(index_file);

    // Put file into index folder
    let crate_file_path = PathBuf::from(format!("{}/{}/{}/download", CONFIG.download.path, index_crate.name.to_lowercase(), index_crate.vers));
    if let Some(parent) = crate_file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_file(crate_file_path, raw_file_bytes)?;

    Ok(add_and_commit_to_index(&index_file_path, &format!("Add package [{}] version [{}] to index", index_crate.name, index_crate.vers))?)
}

fn get_crate_and_raw_bytes_from_stream(stream: &mut TcpStream) -> Result<(IndexCrate, Vec<u8>), ReadStreamError> {
    fn read_number(stream: &mut TcpStream) -> IoResult<[u8; 4]> {
        let mut buf = [0; 4];
        stream.read_exact(&mut buf)?;
        Ok(buf)
    }
    stream.write_all(&Response::new(100).into_bytes())?;

    // u32 notwendig, da usize::from_le sonst nicht 4 Bytes braucht!
    let mut json_buffer = vec![0; u32::from_le_bytes(read_number(stream)?).try_into()?];
    stream.read_exact(&mut json_buffer)?;
    let json = String::from_utf8(json_buffer)?;
    let parsed_json: PublishedPackage = serde_json::from_str(&json)?;

    let mut raw_crate_file = vec![0; u32::from_le_bytes(read_number(stream)?).try_into()?];
    stream.read_exact(&mut raw_crate_file)?;

    let index_crate = IndexCrate::new(parsed_json, &raw_crate_file);
    Ok((index_crate, raw_crate_file))
}

fn write_file(path: PathBuf, raw_bytes: &[u8]) -> PublishResult<()> {
    let mut target_file = File::create(&path)?;
    target_file.write_all(raw_bytes)?;
    println!("Wrote file to {}", path.display());
    Ok(())
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
#[serde(remote = "Self")]
pub(crate) struct PublishedPackage {
    pub(crate) name: String,
    pub(crate) vers: String,
    pub(crate) deps: Vec<Dependency>,
    pub(crate) features: HashMap<String, Vec<String>>,
    pub(crate) authors: Vec<String>,
    pub(crate) description: Option<String>,
    pub(crate) documentation: Option<String>,
    pub(crate) homepage: Option<String>,
    pub(crate) readme: Option<String>,
    pub(crate) readme_file: Option<String>,
    pub(crate) keywords: Vec<String>,
    pub(crate) categories: Vec<String>,
    pub(crate) license: Option<String>,
    pub(crate) license_file: Option<String>,
    pub(crate) repository: Option<String>,
    pub(crate) badges: HashMap<String, String>,
    pub(crate) links: Option<String>,
}

impl<'de> Deserialize<'de> for PublishedPackage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        let mut this = Self::deserialize(deserializer)?;

        if !this.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                return Err(D::Error::custom("non-alphanumeric or -/_ characters allowed!"));
        };
        
        match this.name.chars().next() {
            Some(c) if !c.is_alphabetic() => return Err(D::Error::custom("first character in name must be alphabetic!")),
            None => return Err(D::Error::custom("empty crate name not allowed!")),
            _ => {}
        };

        if this.name.chars().count() > 64 {
            return Err(D::Error::custom("crate name is too long!"))
        }
        
        this.name = this.name.to_ascii_lowercase();
        Ok(this)
    }
}

#[derive(Serialize, Deserialize, Default)]
struct ReturnJson {
    warnings: PublishWarnings
}
#[allow(dead_code)]
impl ReturnJson {
    fn with_others(self, others: Vec<String>) -> Self {
        let mut res = self;
        for o in others {
            res.warnings.other.push(o);
        }
        res
    }
    fn new() -> Self {
        Self::default()
    }
}

#[derive(Serialize, Deserialize, Default)]
struct PublishWarnings {
    invalid_categories: Vec<String>,
    invalid_badges: Vec<String>,
    other: Vec<String>,
}