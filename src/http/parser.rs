use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;

use anyhow::{anyhow, Result};
use crate::http::types::{HttpMethod, HttpRequest, HttpHeaders};

/// Parse a single HTTP request line
fn parse_request_line<R: BufRead>(reader: &mut R) -> Result<(HttpMethod, String, String)> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow!("Empty HTTP request line"));
    }
    if parts.len() < 2 {
        return Err(anyhow!("Missing URI in HTTP request line: '{}'", line));
    }
    if parts.len() < 3 {
        return Err(anyhow!("Missing HTTP version in HTTP request line: '{}'", line));
    }

    let method = parts[0].parse()
        .map_err(|e| anyhow!("Invalid HTTP method '{}': {}", parts[0], e))?;

    let uri = parts[1].to_string();
    if uri.is_empty() {
        return Err(anyhow!("Empty URI in HTTP request line: '{}'", line));
    }

    let http_version = parts[2].to_string();
    if http_version.is_empty() {
        return Err(anyhow!("Empty HTTP version in HTTP request line: '{}'", line));
    }

    Ok((method, uri, http_version))
}

/// Parse HTTP headers from a buffered reader
fn parse_headers<R: BufRead>(reader: &mut R) -> Result<HttpHeaders> {
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
            .ok_or_else(|| anyhow!("Malformed HTTP header: '{}'", line))?;

        headers.insert(key.trim().to_string(), value.trim().to_string());
    }

    Ok(headers)
}

fn parse_body<R: Read>(reader: &mut R, headers: &HttpHeaders) -> Result<Vec<u8>> {
    let body = match get_content_length(&headers)? {
        Some(content_length) => {
            let mut body = vec![0; content_length];
            reader.read_exact(&mut body)?;
            body
        }
        None => {
            Vec::new()
        }
    };
    Ok(body)
}

/// Get the Content-Length from headers
/// Returns None if the header is not present, Some(length) if it is
fn get_content_length(headers: &HttpHeaders) -> Result<Option<usize>> {
    match headers.get("Content-Length") {
        Some(value) => value.parse::<usize>()
            .map(Some)
            .map_err(|_| anyhow!("Invalid Content-Length value: '{}'", value)),
        None => Ok(None),
    }
}

/// Parse a complete HTTP request from a TCP stream
pub fn parse_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let mut reader = BufReader::new(stream);
    let (method, uri, http_version) = parse_request_line(&mut reader)?;
    let headers = parse_headers(&mut reader)?;
    let body = parse_body(&mut reader, &headers)?;
    Ok(HttpRequest::new(method, uri, http_version, headers, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_reader<F, T>(data: &[u8], f: F) -> Result<T>
    where
        F: FnOnce(&mut BufReader<std::io::Cursor<&[u8]>>) -> Result<T>,
    {
        let cursor = std::io::Cursor::new(data);
        let mut reader = BufReader::new(cursor);
        f(&mut reader)
    }

    #[test]
    fn test_parse_request_line_valid() -> Result<()> {
        with_reader(b"GET /index.html HTTP/1.1\r\n", |reader| {
            let (method, uri, version) = parse_request_line(reader)?;
            assert_eq!(method, HttpMethod::Get);
            assert_eq!(uri, "/index.html");
            assert_eq!(version, "HTTP/1.1");
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_line_post() -> Result<()> {
        with_reader(b"POST /api HTTP/1.1\r\n", |reader| {
            let (method, uri, version) = parse_request_line(reader)?;
            assert_eq!(method, HttpMethod::Post);
            assert_eq!(uri, "/api");
            assert_eq!(version, "HTTP/1.1");
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_line_invalid() -> Result<()> {
        with_reader(b"GET /index.html", |reader| {
            assert!(parse_request_line(reader).is_err());
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_line_invalid_method() -> Result<()> {
        with_reader(b"INVALID /path HTTP/1.1\r\n", |reader| {
            let result = parse_request_line(reader);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Invalid HTTP method"));
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_line_empty() -> Result<()> {
        with_reader(b"", |reader| {
            let result = parse_request_line(reader);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Empty HTTP request line"));
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_line_missing_uri() -> Result<()> {
        with_reader(b"GET\r\n", |reader| {
            let result = parse_request_line(reader);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Missing URI"));
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_line_missing_version() -> Result<()> {
        with_reader(b"GET /path\r\n", |reader| {
            let result = parse_request_line(reader);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Missing HTTP version"));
            Ok(())
        })
    }

    #[test]
    fn test_get_content_length_missing() -> Result<()> {
        let headers = HttpHeaders::new();
        assert_eq!(get_content_length(&headers)?, None);
        Ok(())
    }

    #[test]
    fn test_get_content_length_present() -> Result<()> {
        let mut headers = HttpHeaders::new();
        headers.insert("Content-Length", "42");
        assert_eq!(get_content_length(&headers)?, Some(42));
        Ok(())
    }

    #[test]
    fn test_get_content_length_invalid() -> Result<()> {
        let mut headers = HttpHeaders::new();
        headers.insert("Content-Length", "not-a-number");
        assert!(get_content_length(&headers).is_err());
        Ok(())
    }

    #[test]
    fn test_parse_request_get_no_body() -> Result<()> {
        let request_data = b"GET /path HTTP/1.1\r\nHost: example.com\r\n\r\n";

        with_reader(request_data, |reader| {
            let (method, uri, version) = parse_request_line(reader)?;
            let headers = parse_headers(reader)?;
            let body = parse_body(reader, &headers)?;

            assert_eq!(method, HttpMethod::Get);
            assert_eq!(uri, "/path");
            assert_eq!(version, "HTTP/1.1");
            assert_eq!(headers.get("Host"), Some("example.com"));
            assert_eq!(body.len(), 0);
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_post_with_body() -> Result<()> {
        let body_content = b"key1=value1&key2=value2";
        let request = format!(
            "POST /api/data HTTP/1.1\r\nHost: example.com\r\nContent-Length: {}\r\nContent-Type: application/x-www-form-urlencoded\r\n\r\n",
            body_content.len()
        );

        let mut request_bytes = request.into_bytes();
        request_bytes.extend_from_slice(body_content);

        with_reader(&request_bytes, |reader| {
            let (method, uri, _version) = parse_request_line(reader)?;
            let headers = parse_headers(reader)?;
            let body = parse_body(reader, &headers)?;

            assert_eq!(method, HttpMethod::Post);
            assert_eq!(uri, "/api/data");
            assert_eq!(headers.get("Host"), Some("example.com"));
            assert_eq!(headers.get("Content-Length"), Some("23"));
            assert_eq!(body, body_content);
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_no_content_length() -> Result<()> {
        let request_data = b"GET /index HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test-client\r\n\r\n";

        with_reader(request_data, |reader| {
            let (method, _uri, _version) = parse_request_line(reader)?;
            let headers = parse_headers(reader)?;
            let body = parse_body(reader, &headers)?;

            assert_eq!(method, HttpMethod::Get);
            assert_eq!(body.len(), 0); // No Content-Length, so body should be empty
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_zero_content_length() -> Result<()> {
        let request_data = b"POST /api HTTP/1.1\r\nHost: example.com\r\nContent-Length: 0\r\n\r\n";

        with_reader(request_data, |reader| {
            let (method, _uri, _version) = parse_request_line(reader)?;
            let headers = parse_headers(reader)?;
            let body = parse_body(reader, &headers)?;

            assert_eq!(method, HttpMethod::Post);
            assert_eq!(headers.get("Content-Length"), Some("0"));
            assert_eq!(body.len(), 0);
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_multiple_headers() -> Result<()> {
        let request_data = b"PUT /resource HTTP/1.1\r\n\
                             Host: example.com\r\n\
                             User-Agent: TestAgent/1.0\r\n\
                             Content-Type: application/json\r\n\
                             Content-Length: 13\r\n\
                             Accept: */*\r\n\
                             \r\n\
                             {\"key\":\"val\"}";

        with_reader(request_data, |reader| {
            let (method, uri, _version) = parse_request_line(reader)?;
            let headers = parse_headers(reader)?;
            let body = parse_body(reader, &headers)?;

            assert_eq!(method, HttpMethod::Put);
            assert_eq!(uri, "/resource");
            assert_eq!(headers.get("Host"), Some("example.com"));
            assert_eq!(headers.get("User-Agent"), Some("TestAgent/1.0"));
            assert_eq!(headers.get("Content-Type"), Some("application/json"));
            assert_eq!(headers.get("Accept"), Some("*/*"));
            assert_eq!(body, b"{\"key\":\"val\"}");
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_delete() -> Result<()> {
        let request_data = b"DELETE /item/123 HTTP/1.1\r\n\
                             Host: api.example.com\r\n\
                             \r\n";

        with_reader(request_data, |reader| {
            let (method, uri, _version) = parse_request_line(reader)?;
            let headers = parse_headers(reader)?;
            let body = parse_body(reader, &headers)?;

            assert_eq!(method, HttpMethod::Delete);
            assert_eq!(uri, "/item/123");
            assert_eq!(body.len(), 0);
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_binary_body() -> Result<()> {
        let binary_body = &[0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD];
        let request = format!(
            "POST /upload HTTP/1.1\r\nHost: example.com\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\n\r\n",
            binary_body.len()
        );

        let mut request_bytes = request.into_bytes();
        request_bytes.extend_from_slice(binary_body);

        with_reader(&request_bytes, |reader| {
            let (method, uri, _version) = parse_request_line(reader)?;
            let headers = parse_headers(reader)?;
            let body = parse_body(reader, &headers)?;

            assert_eq!(method, HttpMethod::Post);
            assert_eq!(uri, "/upload");
            assert_eq!(body, binary_body);
            Ok(())
        })
    }

    #[test]
    fn test_parse_request_large_body() -> Result<()> {
        let large_body = vec![0x42u8; 10000]; // 10KB of 'B' characters
        let request = format!(
            "POST /large HTTP/1.1\r\nHost: example.com\r\nContent-Length: {}\r\n\r\n",
            large_body.len()
        );

        let mut request_bytes = request.into_bytes();
        request_bytes.extend_from_slice(&large_body);

        with_reader(&request_bytes, |reader| {
            let (method, uri, _version) = parse_request_line(reader)?;
            let headers = parse_headers(reader)?;
            let body = parse_body(reader, &headers)?;

            assert_eq!(method, HttpMethod::Post);
            assert_eq!(uri, "/large");
            assert_eq!(body.len(), 10000);
            assert_eq!(body, large_body);
            Ok(())
        })
    }
}

