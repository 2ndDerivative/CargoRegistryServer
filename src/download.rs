use std::{
    net::TcpStream, 
    path::PathBuf,
    io::{Write, Result as IoResult, ErrorKind},
    fs::read,
};

use crate::{
    error::ReturnJson,
    http::{Response, Byteable}
};

pub fn handle(mut stream: TcpStream, path: &str) -> IoResult<()> {
    let crate_file = PathBuf::from(path.strip_prefix('/').unwrap_or(path));
    let response = match read(crate_file) {
        Ok(file_content) => Response::new(200).body(file_content).into_bytes(),
        Err(e) if e.kind() == ErrorKind::NotFound => Response::new(404).into_bytes(),
        Err(e) => Response::new(500).body(ReturnJson::from(vec![e])).into_bytes(),
    };
    stream.write_all(&response)
}