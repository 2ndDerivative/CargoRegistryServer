use std::{io::{Error as IoError, Write, Read}, net::TcpStream, collections::HashMap};

use serde::{Serialize, Deserialize};

use crate::{ 
    error::ReturnJson,
    database::{self, error::AddOwnerError},
    http::{Response, Byteable}
};

use self::error::UnableToGetUsers;
type IoResult<T> = Result<T, IoError>;

mod error;

pub fn list(mut stream: TcpStream, crate_name: &str, auth: &str) -> IoResult<()> {
    println!("OWNER LIST {crate_name} [{auth}]");
    let users = database::get_owners(crate_name).unwrap();
    let list_result = ListResult { users };
    stream.write_all(&Response::new(200).body(serde_json::to_string(&list_result)?).into_bytes())
}

pub fn add(mut stream: TcpStream, crate_name: &str, headers: &HashMap<String, String>, auth: &str) -> IoResult<()> {
    let users = match get_users_from_stream(&mut stream, headers.get("Content-Length")) {
        Ok(u) => u,
        Err(UnableToGetUsers::IoError(e)) => return Err(e),
        Err(e) => return stream.write_all(&Response::new(400).body(ReturnJson::new(&[e])).into_bytes())
    };
    println!("OWNER ADD {crate_name} [{auth}]");
    match users.iter().try_for_each(|u| database::add_owner(crate_name, u)) {
        Ok(_) => {},
        Err(e) => {
            let code = match &e {
                AddOwnerError::MultipleUsers => 403,
                AddOwnerError::NoSuchUser => 404,
                AddOwnerError::SqlError(_) => 500,
            };
            return stream.write_all(&Response::new(code).body(ReturnJson::new(&[e])).into_bytes());
        }
    }

    let message = format!("Added user{} {} to crate {crate_name}",
        if users.len() > 1 {"s"} else {""},
        users.join(", "));
    stream.write_all(&Response::new(200).body(OkResponse::new(&message)).into_bytes())
}

pub fn remove(mut stream: TcpStream, crate_name: &str, headers: &HashMap<String, String>, auth: &str) -> IoResult<()> {
    let users = match get_users_from_stream(&mut stream, headers.get("Content-Length")) {
        Ok(u) => u,
        Err(UnableToGetUsers::IoError(e)) => return Err(e),
        Err(e) => return stream.write_all(&Response::new(400).body(ReturnJson::new(&[e])).into_bytes())
    };
    println!("OWNER REMOVE {crate_name} [{auth}]");
    for user in &users {
        database::remove_owner(crate_name, user).unwrap();
    }
    let message = format!("Added user{} {} to crate {crate_name}",
        if users.len() > 1 {"s"} else {""},
        users.join(", "));
    stream.write_all(&Response::new(200).body(OkResponse::new(&message)).into_bytes())
}

fn get_users_from_stream(stream: &mut TcpStream, content_length: Option<&String>) -> Result<Vec<String>, UnableToGetUsers> {
    stream.write_all(&Response::new(100).into_bytes())?;
    let mut buffer = vec![0; content_length
        .ok_or(UnableToGetUsers::NoHeaderContentLength)?
        .parse()?];
    stream.read_exact(&mut buffer)?;
    match serde_json::from_str::<Users>(&String::from_utf8(buffer)?) {
        Ok(u) => Ok(u.users),
        Err(e) => Err(UnableToGetUsers::SerdeError(e))
    }
}

#[derive(Serialize, Debug)]
struct ListResult {
    users: Vec<UserResult>
}

#[derive(Serialize, Debug)]
pub (crate) struct UserResult {
    id: u32,
    login: String,
    name: Option<String>
}
impl UserResult {
    pub fn new(id: u32, login: String) -> Self {
        Self { id, login, name: None }
    }
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
    fn new<T: ToString>(msg: &T) -> Self {
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
