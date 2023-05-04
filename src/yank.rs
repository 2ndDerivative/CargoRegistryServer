use std::{
    net::TcpStream,
    fs::OpenOptions,
    io::{Write, Result as IoResult}
};

mod error;

use crate::{
    git::add_and_commit_to_index,
    index::IndexCrate,
    config::CONFIG,
    http::{Response, Byteable}
};

pub(crate) fn unyank(stream: TcpStream, crate_name: &str, version: &str, auth: &str) -> IoResult<()>{
    replace_yanked_field(stream, crate_name, version, auth, false)
}

pub(crate) fn yank(stream: TcpStream, crate_name: &str, version: &str, auth: &str) -> IoResult<()> {
    replace_yanked_field(stream, crate_name, version, auth, true)
}

fn replace_yanked_field(mut stream: TcpStream, crate_name: &str, version: &str, auth: &str, yanked: bool) -> IoResult<()> {
    println!("{} {crate_name} v{version} [{auth}]", 
        if yanked {"YANK"} else {"UNYANK"});

    let index_file_path_relative = IndexCrate {name: crate_name.to_string(), ..Default::default()}.path_in_index();
    let index_file_path_absolute = &CONFIG.index.path.join(&index_file_path_relative);

    let content = std::fs::read_to_string(index_file_path_absolute)?;
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
        .open(index_file_path_absolute)?;

    write!(index_file, "{}\r\n", new_file_content.join("\r\n"))?;
    drop(index_file);

    add_and_commit_to_index(&index_file_path_relative, &format!("{} package [{}] version [{}] from index", 
        if yanked {"Yank"} else {"Unyank"}, crate_name, version))?;
    
    stream.write_all(&Response::new(200).body(r#"{"ok":true}"#).into_bytes())
}