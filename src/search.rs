use std::{io::{Result as IoResult, Error as IoError, ErrorKind, Write}, net::TcpStream, str::FromStr};

use serde::Serialize;
use walkdir::WalkDir;

use crate::{CONFIG, http::{Response, Byteable}, index_crate::IndexCrate, error::ErrorJson};

use self::error::SearchResultError;

mod error;

type CrateVersions = Vec<IndexCrate>;

pub fn handle_search_request(mut stream: TcpStream, path: &str) -> IoResult<()> {
    let query = match path.parse::<Query>() {
        Ok(query) => query,
        Err(e) => return stream.write_all(&Response::new(400).body(ErrorJson::new(&[e])).into_bytes())
    };

    let crate_files = WalkDir::new(&CONFIG.index.path).into_iter().flatten().filter(|p| 
        p.path().is_file() 
        && !p.path().starts_with(CONFIG.index.path.join(".git")) 
        && p.path() != CONFIG.index.path.join("config.json")
    );

    // Check ob alle Dateien gelesen werden konnten
    // Check if all Files could be read to strings
    let crates_file_strings = crate_files.map(|file| {
            std::fs::read_to_string(file.path())
        }).collect::<Result<Vec<String>,_>>()?;

    // Check ob alle gelesenen Strings geparsed werden konnten
    // Check if all read strings could be parsed
    let crate_versions = crates_file_strings
        .into_iter()
        .map(|filestring| {
            filestring.lines()
                .map(serde_json::from_str::<IndexCrate>)
                .collect()
        }).collect::<Result<Vec<CrateVersions>, _>>()?;

    // Alle übrigen Gruppen zu Suchergebnissen umwandeln
    // TryFrom handlet auch Crates die nur Yanked versionen haben
    // Flattening all crates to search result elements
    let crate_groups = crate_versions.into_iter()
        .filter_map(|u| 
            match u.try_into() {
                Ok(x) => Some(Ok(x)),
                // Ein Fehler durch das Umwandeln eines leeren Vektors kann als fehlender Vektor gesehen werden
                // An error caused by an empty Vector can be passed as "no search result"
                Err(SearchResultError::EmptyVector) => None,
                // Parsingfehler müssen allerdings durchgereicht werden!
                // A parsing mistake has to be handled and will pass as a 500 here
                Err(e) => Some(Err(IoError::new(ErrorKind::InvalidData, e))),
            }
        )
        .collect::<Result<Vec<SearchResult>,_>>()?;

    let results_matching_query = crate_groups.into_iter()
        .filter(|i: &SearchResult| i.name.contains(&query.query_string.replace('-',"_").to_ascii_lowercase()))
        .collect::<Vec<_>>();

    let crates: Vec<_> = results_matching_query.into_iter().take(query.per_page.min(100)).collect();
    let results_json = Results {
        meta: Meta { total: crates.len() },
        crates,
    };
    let response = Response::new(200).body(serde_json::to_string(&results_json)?);
    stream.write_all(&response.into_bytes())
}

#[derive(Debug)]
struct Query {
    query_string: String,
    per_page: usize,
}

impl FromStr for Query {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('&');
        let (query_side, per_page_side) = match (split.next(), split.next()) {
            (Some(a), Some(b)) => (a, b),
            _ => return Err("No two query elements found")
        };
        let query_string = match query_side.strip_prefix("?q=") {
            Some(q) => q.to_string(),
            _ => return Err("No query string parsed")
        };
        let per_page = match per_page_side.strip_prefix("per_page=") {
            Some(o) => if let Ok(p) = o.parse() {
                p
            } else {
                return Err("per_page is not a number")
            }
            _ => return Err("No per_page string parsed")
        };
        Ok(Query {query_string, per_page})
    }
}

#[derive(Serialize)]
struct Results {
    crates: Vec<SearchResult>,
    meta: Meta,
}

#[derive(Serialize)]
struct Meta {
    total: usize,
}

#[derive(Serialize)]
struct SearchResult {
    name: String,
    max_version: String,
    description: String,
}

impl TryFrom<Vec<IndexCrate>> for SearchResult {
    type Error = SearchResultError;

    fn try_from(value: Vec<IndexCrate>) -> Result<Self, Self::Error> {
        let name = if value.iter()
            .all(|i| i.name==value[0].name) {
                value[0].name.clone()
            } else {
                return Err(SearchResultError::EmptyVector)
            };
        let max_version = value.into_iter()
            .filter(|i| !i.yanked)
            .map(|x| {
                let mut splittage = x.vers.split('.')
                    .take(3)
                    .map(|s| s.parse::<u32>())
                .collect::<Result<Vec<_>,_>>()?
                .into_iter();
                Ok((
                    splittage.next().ok_or(SearchResultError::MissingVersionElements)?,
                    splittage.next().ok_or(SearchResultError::MissingVersionElements)?,
                    splittage.next().ok_or(SearchResultError::MissingVersionElements)?,
                ))
            })
            .collect::<Result<Vec<(u32, u32, u32)>,SearchResultError>>()?
            .into_iter()
            .max()
            .map(|(a, b, c)| format!("{a}.{b}.{c}"))
            .ok_or(SearchResultError::EmptyVector)?;
        Ok(SearchResult { name, max_version, description: String::new() })
    }
}