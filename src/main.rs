#![warn(clippy::pedantic)]
use std::{
    net::{TcpListener, TcpStream, SocketAddr}, 
    io::{BufReader, BufRead, Write, Result as IoResult}, 
    fs::create_dir_all, collections::HashMap,
    error::Error,
};

use threads::ThreadPool;
use http::{Request, Response, RequestMethod, Byteable};
use config::CONFIG;
use error::ReturnJson;

mod http;
mod threads;
mod publish;
mod download;
mod yank;
mod search;
mod index;
mod dependency;
mod git;
mod config;
mod error;
mod owners;
mod database;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting up!");
    let index_path = &CONFIG.index.path;
    let socket_addr = SocketAddr::new(CONFIG.net.ip, CONFIG.net.port);
    if index_path.exists() {
        println!("Using existing index at {}", index_path.display());
    } else {
        println!("Creating new index at configured path {}", index_path.display());
        create_dir_all(index_path)?;

        git::init_index()?;
        index::write_config_json(index_path)?;
        git::add_and_commit_to_index(&"config.json", "Init index")?;
    }
    let database_path = &CONFIG.database.path;
    if database_path.is_file() {
        println!("Using existing database file at {}", database_path.display());
    } else {
        println!("Creating new database at configured path {}", database_path.display());
        database::init(database_path.as_path())?; 
    }

    let listener = TcpListener::bind(socket_addr)?;
    println!("Binding to {socket_addr}");
    let pool = ThreadPool::new(CONFIG.net.threads.unwrap_or(10));

    for stream in listener.incoming() {
        let stream = stream.expect("connection failed!");

        pool.execute(|| {
            handle_connection(stream).expect("stream interrupted");
        });
    };
    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> IoResult<()> {
    let buffer = BufReader::new(&mut stream);
    let request: String = buffer.lines()
        .map(Result::unwrap)
        .take_while(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\r\n");
    println!("Connection with request:\n{request}");
    match request.parse() {
        Ok(request) => handle_request(request, stream),
        Err(e) => {
            println!("Request not recognized!");
            stream.write_all(&Response::new(400).body(ReturnJson::new(&[e])).into_bytes())
        }
    }
}

pub const API_COMMON: [&str; 3] = ["api", "v1", "crates"];

fn handle_request(request: Request, mut stream: TcpStream) -> IoResult<()> {
    let dl_pattern: Vec<_> = CONFIG.download.path.split('/').collect();
    let Request{method, path, headers} = request;
    match (method, &path.split('/').collect::<Vec<_>>()[1..]) {
        (RequestMethod::Get, [rest @ .., name, version, "download"]) if rest == dl_pattern => {
            println!("DOWNLOAD {name} v{version}");
            download::handle(stream, &path)},

        (RequestMethod::Put, [rest @ .., "new"]) if rest==API_COMMON => handle_authorized(stream, headers, |s, _, a| publish::handle_publish_request(s, a)),

        (RequestMethod::Put, [rest @ .., crate_name, version, "unyank"]) if rest == API_COMMON => handle_authorized(stream, headers, |s, _, a| yank::unyank(s, crate_name, version, a)),
        (RequestMethod::Delete, [rest @ .., crate_name, version, "yank"]) if rest == API_COMMON => handle_authorized(stream, headers, |s, _, a| yank::yank(s, crate_name, version, a)),

        (RequestMethod::Get, [rest @ .., crate_name, "owners"]) if rest == API_COMMON => handle_authorized(stream, headers, |s, _, a| owners::list(s, crate_name, a)),
        (RequestMethod::Put, [rest @ .., crate_name, "owners"]) if rest == API_COMMON => handle_authorized(stream, headers, |s, h, a| owners::add(s, crate_name, &h, a)),
        (RequestMethod::Delete, [rest @ .., crate_name, "owners"])if rest == API_COMMON => handle_authorized(stream, headers, |s, h, a| owners::remove(s, crate_name, &h, a)),

        (RequestMethod::Get, ["api", "v1", query]) if query.starts_with("crates?") => search::handle_search_request(stream, query.strip_prefix("crates").unwrap()),
        (method, _) => {
            println!("Unrecognized {method:?} request for {path}");
            stream.write_all(&Response::new(405).into_bytes())
        }
    }
}

fn handle_authorized<F>(mut stream: TcpStream, mut headers: HashMap<String, String>, f: F) -> IoResult<()> 
    where F: Fn(TcpStream, HashMap<String, String>, &str) -> IoResult<()>{
    if let Some(auth) = headers.remove("Authorization") {
        f(stream, headers, &auth)
    } else {
        stream.write_all(
            &Response::new(401).body(ReturnJson::new(&["missing authorization token"]))
                .into_bytes())
    }
}