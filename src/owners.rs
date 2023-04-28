use std::{io::{Error as IoError, Write, Read}, net::TcpStream, collections::HashMap};

use serde::{Serialize, Deserialize};

use crate::{http::{Response, Byteable}, API_COMMON};
type IoResult<T> = Result<T, IoError>;

const NOT_IMPLEMENTED_WARNING: &str = r#"{"errors":[{"detail":"Sorry, this functionality is not implemented yet :("}]}"#;

pub fn list(mut stream: TcpStream, path: &str) -> IoResult<()> {
    let _crate_name = get_crate_name(path);
    stream.write_all(&Response::new(501).body(NOT_IMPLEMENTED_WARNING).into_bytes())
}

pub fn add(mut stream: TcpStream, path: &str, headers: HashMap<String, String>) -> IoResult<()> {
    stream.write_all(&Response::new(100).into_bytes())?;
    let crate_name = get_crate_name(path);
    let mut buffer = vec![0; headers.get("Content-Length").unwrap().parse().unwrap()];
    stream.read_exact(&mut buffer)?;
    let add_request: Users = serde_json::from_str(&String::from_utf8(buffer).unwrap()).unwrap();
    let users = add_request.users;
    let message = format!("Added user{} {} to crate {crate_name}", if users.len() > 1 {"s"} else {""}, users.join(", "));
    stream.write_all(&Response::new(200).body(OkResponse::new(message)).into_bytes())
}

pub fn remove(mut stream: TcpStream, path: &str, headers: HashMap<String, String>) -> IoResult<()> {
    stream.write_all(&Response::new(100).into_bytes())?;
    let mut buffer = vec![0; headers.get("Content-Length").unwrap().parse().unwrap()];
    stream.read_exact(&mut buffer)?;
    let users: Users = serde_json::from_str(&String::from_utf8(buffer).unwrap()).unwrap();
    let users = users.users;
    let crate_name = get_crate_name(path);
    let message = format!("Removed user{} {} from crate {crate_name}", if users.len() > 1 {"s"} else {""}, users.join(", "));
    stream.write_all(&Response::new(200).body(OkResponse::new(message)).into_bytes())
}

fn get_crate_name(path: &str) -> &str {
    path.strip_prefix(&format!("{}/", API_COMMON))
        .and_then(|x| x.strip_suffix("/owners"))
        .expect("Have been checked in main module")
}

#[derive(Serialize, Debug)]
struct ListResult {
    users: Vec<UserResult>
}

#[derive(Serialize, Debug)]
struct UserResult {
    id: u32,
    login: String,
    name: Option<String>
}

#[derive(Deserialize, Debug)]
struct Users {
    users: Vec<String>
}

#[derive(Serialize, Debug)]
struct OkResponse {
    ok: bool,
    msg: String
}

impl OkResponse {
    fn new<T: ToString>(msg: T) -> Self {
        Self {
            ok: true,
            msg: msg.to_string()
        }
    }
}

impl From<OkResponse> for Vec<u8> {
    fn from(value: OkResponse) -> Self {
        serde_json::to_string(&value).unwrap().into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::get_crate_name;
    #[test]
    fn crate_name_from_path() {
        let requested_resource = "/api/v1/crates/test_crate/owners";
        assert_eq!(get_crate_name(requested_resource), "test_crate");
    }
    #[test]
    #[should_panic]
    fn crate_name_refuse_no_order() {
        let requested_resource = "/api/v1/crates/stuff";
        let _ = get_crate_name(requested_resource);
    }
}