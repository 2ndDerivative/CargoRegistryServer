use std::{
    io::{Read, Write, Result as IoResult},
    net::TcpStream, 
    collections::HashMap, 
    path::PathBuf, 
    fs::{OpenOptions, File},
};
use crate::{
    index::{IndexCrate, self}, 
    dependency::Dependency, 
    git::add_and_commit_to_index,  
    error::ReturnJson as ErrorJson, 
    config::CONFIG, database,
    http::{Response, Byteable},
};
use serde::{Deserialize, Serialize, de::Error};

use self::error::{PublishError, ReadStreamError};

pub mod error;
type PublishResult<T> = core::result::Result<T, PublishError>;

pub(crate) fn handle_publish_request(mut stream: TcpStream, auth: &str) -> IoResult<()> {
    let (published_crate, raw_crate_file) = match get_crate_and_raw_bytes_from_stream(&mut stream) {
        Ok(t) => t,
        Err(e) => {
            use ReadStreamError::{
                BadHTTPJson, ConnectionClosed, 
                InvalidUTF8Error, NonNumericContentLength, PayloadTooLarge
            };
            let code = match e {
                ConnectionClosed(e) => return Err(e),
                BadHTTPJson(_) | NonNumericContentLength(_) | InvalidUTF8Error(_) => 400,
                PayloadTooLarge => 413,
            };
            let response = Response::new(code).body(ErrorJson::new(&[e]));
            return stream.write_all(&response.into_bytes())
        }
    };
    println!("PUBLISH {} v{} [{auth}]", published_crate.name, published_crate.vers);
    
    match process_publish_request(&published_crate, &raw_crate_file) {
        Ok(_) => {
            let warnings_json = serde_json::to_string(
                &ReturnJson::new()).expect("This is a static json object");
            Ok(stream.write_all(&Response::new(200).body(warnings_json).into_bytes())?)
        },
        Err(pub_err) => {
            use PublishError::{
                BadIndexJson, CrateExistsWithDifferentDashUnderscore, 
                IoError, SerializationFailed, VersionAlreadyExists
            };
            let code = match pub_err {
                VersionAlreadyExists | CrateExistsWithDifferentDashUnderscore => 403,
                IoError(_) | BadIndexJson | SerializationFailed(_) => 500,
            };
            let response = Response::new(code).body(ErrorJson::new(&[pub_err]));
            stream.write_all(&response.into_bytes())
        }
    }
}

fn process_publish_request(package: &PublishedPackage, raw_file_bytes: &[u8]) -> PublishResult<()> {
    let index_crate = IndexCrate::new(package.clone(), raw_file_bytes);
    // Check for existing version
    let index_file_path_absolute = &CONFIG.index.path.join(index_crate.path_in_index());
    for index_crate_res in index::walk_index_crates() {
        let index_crate_in_file = index_crate_res?;
        if index_crate_in_file.name.replace('-', "_") == index_crate.name.replace('-', "_") {
            if index_crate_in_file.name != index_crate.name {
                return Err(PublishError::CrateExistsWithDifferentDashUnderscore)
            } else if index_crate_in_file.vers == index_crate.vers {
                return Err(PublishError::VersionAlreadyExists)
            }
        }
    }

    database::add_package(package).unwrap();

    if !index_file_path_absolute.exists() {
        if let Some(parent) = index_file_path_absolute.parent() {
            std::fs::create_dir_all(parent)?;
        }
    }
    
    let mut index_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(index_file_path_absolute)?;

    write!(index_file, "{}\r\n", serde_json::to_string(&index_crate)?)?;
    drop(index_file);

    // Put file into index folder
    let crate_file_path = PathBuf::from(format!("{}/{}/{}/download", CONFIG.download.path, index_crate.name.to_lowercase(), index_crate.vers));
    if let Some(parent) = crate_file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_file(&crate_file_path, raw_file_bytes)?;

    Ok(add_and_commit_to_index(&index_crate.path_in_index(), &format!("Add package [{}] version [{}] to index", index_crate.name, index_crate.vers))?)
}

fn get_crate_and_raw_bytes_from_stream(stream: &mut TcpStream) -> Result<(PublishedPackage, Vec<u8>), ReadStreamError> {
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

    Ok((parsed_json, raw_crate_file))
}

fn write_file(path: &PathBuf, raw_bytes: &[u8]) -> PublishResult<()> {
    let mut target_file = File::create(path)?;
    target_file.write_all(raw_bytes)?;
    println!("Wrote file to {}", path.display());
    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
#[serde(remote = "Self")]
/// "name" is already normalized, so you can insert it into the database or use it for search terms
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