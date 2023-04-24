use std::{
    net::{TcpListener, TcpStream}, 
    io::{BufReader, BufRead, Write, Result as IoResult}, 
    fs::{create_dir_all, File, OpenOptions},
};

use threads::ThreadPool;

use http::{Request, Response};
use config::CONFIG;

use crate::http::{RequestMethod, Byteable};

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

fn main() -> IoResult<()> {
    println!("Starting up!");
    let index_path = &CONFIG.index.path;
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
        index_config.write_all(
            format!("{{\r\n\"dl\": \"http://{0}/{1}\",\r\n\"api\": \"http://{0}\"\r\n}}", CONFIG.net.ip, CONFIG.download.path).as_bytes()
        )?;
        drop(index_config);
        git::init_index()?;
        git::add_and_commit_to_index(&index_config_path, "Init index")?;
    }
    let listener = TcpListener::bind(CONFIG.net.ip)?;
    println!("Binding to {}", CONFIG.net.ip);
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
    if let Ok(request) = request.parse() {
        handle_request(request, stream)
    } else {
        stream.write_all(&Response::new(400).into_bytes())
    }
}

const API_COMMON: &str = "/api/v1/crates";
const API_NEW: &str = "/api/v1/crates/new";

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

fn error_json(errors: &[&str]) -> Vec<u8> {
    format!("{{\"errors\":[{}]}}", 
        errors.iter().map(|err|
            format!("{{\"detail\":\"{err}\"}}")
        ).collect::<Vec<_>>().join(",")).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::error_json;
    #[test]
    fn error_json_two_args() {
        let words = ["haha", "hehe"];
        let json = error_json(&words);
        assert_eq!(json,
            r#"{"errors":[{"detail":"haha"},{"detail":"hehe"}]}"#.as_bytes()
        );
    }
}