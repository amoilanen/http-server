use std::io::{BufRead, BufReader, Error, ErrorKind, Read};
use std::net::TcpStream;
use std::str::FromStr;

use crate::http::types::{HttpMethod, HttpRequest, HttpHeaders};

/// Parse a single HTTP request line
fn parse_request_line(line: &str) -> Result<(HttpMethod, String, String), Error> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 3 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Malformed HTTP request line: '{}'", line),
        ));
    }

    let method = HttpMethod::from_str(parts[0])
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

    let uri = parts[1].to_string();
    let http_version = parts[2].to_string();

    Ok((method, uri, http_version))
}

/// Parse HTTP headers from a buffered reader
fn parse_headers(reader: &mut BufReader<&mut TcpStream>) -> Result<HttpHeaders, Error> {
    let mut headers = HttpHeaders::new();
    let mut line = String::new();

    loop {
        line.clear();
        reader.read_line(&mut line)?;

        if line == "\r\n" || line.is_empty() {
            break;
        }

        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| Error::new(
                ErrorKind::InvalidData,
                format!("Malformed HTTP header: '{}'", line),
            ))?;

        headers.insert(key.trim().to_string(), value.trim().to_string());
    }

    Ok(headers)
}

/// Get the Content-Length from headers, defaults to 0 if not present
fn get_content_length(headers: &HttpHeaders) -> Result<usize, Error> {
    match headers.get("Content-Length") {
        Some(value) => value.parse::<usize>().map_err(|_| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Invalid Content-Length value: '{}'", value),
            )
        }),
        None => Ok(0),
    }
}

/// Parse a complete HTTP request from a TCP stream
pub fn parse_request(stream: &mut TcpStream) -> Result<HttpRequest, Error> {
    let mut reader = BufReader::new(stream);

    // Parse request line
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let (method, uri, http_version) = parse_request_line(&request_line)?;

    // Parse headers
    let headers = parse_headers(&mut reader)?;

    // Read body based on Content-Length
    let content_length = get_content_length(&headers)?;
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;

    Ok(HttpRequest::new(method, uri, http_version, headers, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request_line_valid() {
        let (method, uri, version) = parse_request_line("GET /index.html HTTP/1.1\r\n").unwrap();
        assert_eq!(method, HttpMethod::Get);
        assert_eq!(uri, "/index.html");
        assert_eq!(version, "HTTP/1.1");
    }

    #[test]
    fn test_parse_request_line_post() {
        let (method, uri, version) = parse_request_line("POST /api HTTP/1.1\r\n").unwrap();
        assert_eq!(method, HttpMethod::Post);
        assert_eq!(uri, "/api");
        assert_eq!(version, "HTTP/1.1");
    }

    #[test]
    fn test_parse_request_line_invalid() {
        assert!(parse_request_line("GET /index.html").is_err());
        assert!(parse_request_line("INVALID /path HTTP/1.1").is_err());
    }

    #[test]
    fn test_get_content_length_missing() {
        let headers = HttpHeaders::new();
        assert_eq!(get_content_length(&headers).unwrap(), 0);
    }

    #[test]
    fn test_get_content_length_present() {
        let mut headers = HttpHeaders::new();
        headers.insert("Content-Length", "42");
        assert_eq!(get_content_length(&headers).unwrap(), 42);
    }

    #[test]
    fn test_get_content_length_invalid() {
        let mut headers = HttpHeaders::new();
        headers.insert("Content-Length", "not-a-number");
        assert!(get_content_length(&headers).is_err());
    }
}

