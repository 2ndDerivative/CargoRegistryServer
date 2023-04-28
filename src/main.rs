use std::{
    net::{TcpListener, TcpStream, SocketAddr}, 
    io::{BufReader, BufRead, Write, Result as IoResult}, 
    fs::{create_dir_all, File, OpenOptions},
};

use threads::ThreadPool;

use http::{Request, Response};
use config::{CONFIG, IndexConfigFile};

use crate::{http::{RequestMethod, Byteable}, error::ErrorJson};

mod threads;
mod http;
mod publish;
mod download;
mod yank;
mod search;
mod index_crate;
mod dependency;
mod git;
mod config;
mod error;

fn main() -> IoResult<()> {
    println!("Starting up!");
    let index_path = &CONFIG.index.path;
    let socket_addr = SocketAddr::new(CONFIG.net.ip, CONFIG.net.port);
    if index_path.exists() {
        println!("Using existing index at {}", index_path.display());
    } else {
        // Wenn kein Index vorhanden ist, wird ein neuer erstellt,
        // eine config.json aus der aktuellen Config erstellt und
        // committet.
        println!("Creating new index at configured path {}", index_path.display());
        create_dir_all(index_path)?;
        let index_config_path = index_path.join("config.json");
        File::create(&index_config_path)?;
        let mut index_config = OpenOptions::new().write(true).open(&index_config_path)?;
        let cfg: IndexConfigFile = CONFIG.clone().try_into().expect("bad configured URL");
        index_config.write_all(serde_json::to_string_pretty(&cfg)?.as_bytes())?;
        drop(index_config);
        git::init_index()?;
        git::add_and_commit_to_index(&index_config_path, "Init index")?;
    }

    let listener = TcpListener::bind(socket_addr)?;
    println!("Binding to {}", socket_addr);
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
            stream.write_all(&Response::new(400).body(ErrorJson::new(&[e])).into_bytes())
        }
    }
}

pub const API_COMMON: &str = "/api/v1/crates";
pub const API_NEW: &str = "/api/v1/crates/new";

fn handle_request(request: Request, mut stream: TcpStream) -> IoResult<()> {
    match request {
        Request{method: RequestMethod::Get, path, ..} if path.strip_prefix('/').unwrap_or(&path).starts_with(&CONFIG.download.path) 
            && path.ends_with("download") => download::handle_download_request(stream, &path),
        Request{method: RequestMethod::Put, path, ..} if path == API_NEW => publish::handle_publish_request(stream),
        Request{method: RequestMethod::Put, path, ..} if path.starts_with(API_COMMON) && path.ends_with("/unyank") => yank::unyank(stream, &path),
        Request{method: RequestMethod::Delete, path, ..} if path.starts_with(API_COMMON) && path.ends_with("/yank") => yank::yank(stream, &path),
        Request{method: RequestMethod::Get, path, ..} if path.starts_with(API_COMMON) => search::handle_search_request(stream, path.strip_prefix(API_COMMON).expect("if let guard unstable")),
        Request{method: RequestMethod::Get | RequestMethod::Put | RequestMethod::Delete, path, ..} 
            if path.starts_with(API_COMMON) && path.ends_with("owners") => stream.write_all(&Response::new(501).into_bytes()),
        _ => stream.write_all(&Response::new(405).into_bytes())
    }
}