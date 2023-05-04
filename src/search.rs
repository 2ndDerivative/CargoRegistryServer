use std::{io::{Result as IoResult, Error as IoError, ErrorKind, Write}, net::TcpStream, str::FromStr};

use serde::Serialize;

use crate::{
    index::{IndexCrate, self}, 
    error::ReturnJson,
    database,
    http::{Response, Byteable}
};

use self::error::SearchResultError;

mod error;

type CrateVersions = Vec<IndexCrate>;

pub fn handle_search_request(mut stream: TcpStream, path: &str) -> IoResult<()> {
    let query = match path.parse::<Query>() {
        Ok(query) => query,
        Err(e) => return stream.write_all(&Response::new(400).body(ReturnJson::new(&[e])).into_bytes())
    };

    let crates = index::walk_index_crates();
    let crate_versions: Vec<CrateVersions> = crates.fold(Ok(vec![]), |vec: Result<Vec<Vec<IndexCrate>>, IoError>, new_crate| {
        let (mut vector, new_crate) = match (vec, new_crate) {
            (Ok(x), Ok(y)) => (x, y),
            (v, _)  => return v
        };
        let Some(last_crate_name) = vector.last()
            .and_then(|v| v.first())
            .map(|i| i.name.clone()) else {
            return Ok(vec![vec![new_crate]]);
        };
        if last_crate_name == new_crate.name {
            vector.last_mut().expect("should be initialized non-empty").push(new_crate);
            Ok(vector)
        } else {
            vector.push(vec![new_crate]);
            Ok(vector)
        }
    })?;

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
        .filter(|i: &SearchResult| {
            let desc = database::get_description(&i.name, &i.max_version).unwrap().map(|x| x.to_ascii_lowercase());
            i.name.contains(&query.query_string.replace('-',"_").to_ascii_lowercase())
            || (desc.is_some() && desc.unwrap().contains(&query.query_string.to_ascii_lowercase()))
        })
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
        let (Some(query_side), Some(per_page_side)) = (split.next(), split.next()) else {
            return Err("No two query elements found")
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
                    .map(str::parse)
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
        let description = database::get_description(&name, &max_version).unwrap().unwrap_or_default();
        Ok(SearchResult { name, max_version, description })
    }
}