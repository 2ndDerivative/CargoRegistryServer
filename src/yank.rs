use std::{net::TcpStream, path::{PathBuf, Path}, fs::OpenOptions};
use std::io::{Write, Result as IoResult};

mod error;

use crate::{index_crate::IndexCrate, http::{Response, Byteable}, error_json};
use crate::git::add_and_commit_to_index;

use self::error::YankPathError;

pub(crate) fn unyank(stream: TcpStream, path: &str) -> IoResult<()>{
    replace_yanked_field(stream, &PathBuf::from(path), false)
}

pub(crate) fn yank(stream: TcpStream, path: &str) -> IoResult<()> {
    replace_yanked_field(stream, &PathBuf::from(path), true)
}

fn replace_yanked_field(mut stream: TcpStream, path: &Path, yanked: bool) -> IoResult<()> {
    let (version, name) = match parse_yank_path(path) {
        Err(e) => {
            let body = error_json(&[&e.to_string()]);
            return stream.write_all(&Response::new(400).body(body).into_bytes())
        },
        Ok(t) => t,
    };

    let index_file_path = IndexCrate {
        name: name.to_string(), 
        ..Default::default()}.path();
    let content = std::fs::read_to_string(&index_file_path)?;
    let mut new_file_content = content.lines().map(ToString::to_string).collect::<Vec<_>>();
    let index: Option<usize> = new_file_content.iter()
        .enumerate()
        .find_map(|(i, x)| {
            let icrate = serde_json::from_str::<IndexCrate>(x).ok()?;
            (icrate.vers == version).then_some(i)
        });
    
    if let Some(index) = index {
        new_file_content[index] = new_file_content.get(index).expect("just parsed, cannot be empty").replace(
            &format!("\"yanked\":{}", !yanked), 
            &format!("\"yanked\":{yanked}"));
    };

    let mut index_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&index_file_path)?;

    write!(index_file, "{}\r\n", new_file_content.join("\r\n"))?;
    drop(index_file);

    add_and_commit_to_index(&index_file_path, &format!("{} package [{}] version [{}] from index", 
        if yanked {"Yank"} else {"Unyank"}, name, version))?;
    
    stream.write_all(&Response::new(200).body(r#"{"ok":true}"#).into_bytes())
}

fn parse_yank_path(path: &Path) -> Result<(&str, &str), YankPathError> {
    let mut elements = path.ancestors().skip(1);
    let version = elements.next().ok_or(YankPathError::NoVersion)?
        .file_name().ok_or(YankPathError::DotDotInPath)?
        .to_str().ok_or(YankPathError::InvalidUTF8Error)?;
    let name = elements.next().ok_or(YankPathError::NoName)?
        .file_name().ok_or(YankPathError::DotDotInPath)?
        .to_str().ok_or(YankPathError::InvalidUTF8Error)?;
    Ok((version, name))
}