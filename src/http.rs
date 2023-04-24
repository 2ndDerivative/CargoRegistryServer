use std::collections::HashMap;
use std::fmt::{Formatter, Result as FMTResult, Display};
use std::marker::PhantomData;
use std::str::FromStr;
use std::error::Error;

#[derive(Debug)]
pub struct Request {
    pub method: RequestMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug)]
pub enum RequestMethod {
    Get,
    Delete,
    Put
}

#[derive(Debug)]
pub enum RequestParseError {
    EmptyRequest,
    NoMethod,
    NoPath,
    NoHttpWord,
    InvalidMethodWord,
}
impl Error for RequestParseError{}
impl Display for RequestParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FMTResult {
        write!(f, "failed to parse HTTP Request!")
    }
}

impl FromStr for Request {
    type Err = RequestParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use RequestMethod::{Delete, Get, Put};
        let mut lines = s.lines();
        let mut firstline = lines.next().ok_or(RequestParseError::EmptyRequest)?.split_whitespace();
        let method_word = firstline.next().ok_or(RequestParseError::NoMethod)?;
        let path = firstline.next().ok_or(RequestParseError::NoPath)?.to_string();
        let _http_word = firstline.next().ok_or(RequestParseError::NoHttpWord)?;
        let headers = lines
            .take_while(|l| !l.is_empty())
            .filter_map(|f| {
                let mut parts = f.split(':');
                Some((parts.next()?.trim().to_string(), parts.next()?.trim().to_string()))
            })
            .collect();
        if method_word.starts_with("GET") {
            Ok(Request{method: Get, path, headers})
        } else if method_word.starts_with("PUT") {
            Ok(Request{method: Put, path, headers})
        } else if method_word.starts_with("DELETE") {
            Ok(Request{method: Delete, path, headers})
        } else {
            Err(RequestParseError::InvalidMethodWord)
        }
    }
}

pub struct Response<S: State> {
    marker: std::marker::PhantomData<S>,
    front_matter: String,
    body: Vec<u8>,
}

impl Response<NeedsHeaders> {
    pub fn new(code: u32) -> Self {
        Response {
            marker: PhantomData,
            front_matter: format!("HTTP/1.1 {code} {}", 
                standard_marker_from_code(code)
            ),
            body: vec![], 
        }
    }
    pub fn body<B: Into<Vec<u8>>>(self, body: B) -> Response<NeedsMessage> {
        Response { body: body.into(), marker: PhantomData, front_matter: self.front_matter }
    }
}

impl<S: State> Byteable for Response<S> {
    fn into_bytes(self) -> Vec<u8> {
        [self.front_matter.into_bytes(),"\r\n\r\n".into(), self.body].concat()
    }
}

pub trait Byteable {
    fn into_bytes(self) -> Vec<u8>;
}

fn standard_marker_from_code(code: u32) -> &'static str {
    match code {
        100 => "CONTINUE",

        200 => "OK",

        400 => "BAD REQUEST",
        403 => "FORBIDDEN",
        405 => "METHOD NOT ALLOWED",
        411 => "LENGTH REQUIRED",
        413 => "PAYLOAD TOO LARGE",

        500 => "SERVER ERROR",
        501 => "NOT IMPLEMENTED",
        _ => todo!(),
    }
}

pub enum NeedsHeaders {}
impl State for NeedsHeaders {}
pub enum NeedsMessage {}
impl State for NeedsMessage {}
pub trait State {}

#[cfg(test)]
mod tests {
    use super::{Response, Byteable};

    #[test]
    fn response_ok_bytes() {
        let result = Response::new(200).into_bytes();
        assert_eq!(result, b"HTTP/1.1 200 OK\r\n\r\n");
    }
    #[test]
    fn response_body_correct() {
        let result = Response::new(200).body("SomeBODY");
        assert_eq!(result.into_bytes(), b"HTTP/1.1 200 OK\r\n\r\nSomeBODY");
    }
}