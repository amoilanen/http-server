use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// HTTP request methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        }
    }
}

impl FromStr for HttpMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "DELETE" => Ok(HttpMethod::Delete),
            _ => Err(format!("Unknown HTTP method: {}", s)),
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// HTTP request headers - case-insensitive key lookup
#[derive(Debug, Clone)]
pub struct HttpHeaders {
    headers: HashMap<String, String>,
}

impl HttpHeaders {
    pub fn new() -> Self {
        HttpHeaders {
            headers: HashMap::new(),
        }
    }

    /// Add a header (key-value pair)
    pub fn insert<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.headers.insert(key.into().to_lowercase(), value.into());
    }

    /// Get a header value (case-insensitive)
    pub fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(&key.to_lowercase()).map(|v| v.as_str())
    }

    /// Check if a header exists
    #[allow(dead_code)]
    pub fn contains(&self, key: &str) -> bool {
        self.headers.contains_key(&key.to_lowercase())
    }

    /// Get all headers as an iterator
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.headers.iter()
    }
}

impl Default for HttpHeaders {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub uri: String,
    #[allow(dead_code)]
    pub http_version: String,
    pub headers: HttpHeaders,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn new(
        method: HttpMethod,
        uri: String,
        http_version: String,
        headers: HttpHeaders,
        body: Vec<u8>,
    ) -> Self {
        HttpRequest {
            method,
            uri,
            http_version,
            headers,
            body,
        }
    }
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub http_version: String,
    pub status: u16,
    pub reason_phrase: String,
    pub headers: HttpHeaders,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(
        status: u16,
        reason_phrase: impl Into<String>,
        headers: HttpHeaders,
        body: Vec<u8>,
    ) -> Self {
        HttpResponse {
            http_version: "HTTP/1.1".to_string(),
            status,
            reason_phrase: reason_phrase.into(),
            headers,
            body,
        }
    }

    pub fn ok(headers: HttpHeaders, body: Vec<u8>) -> Self {
        Self::new(200, "OK", headers, body)
    }

    pub fn created(headers: HttpHeaders, body: Vec<u8>) -> Self {
        Self::new(201, "Created", headers, body)
    }

    pub fn not_found() -> Self {
        Self::new(404, "Not Found", HttpHeaders::new(), Vec::new())
    }

    /// Format the status line and headers as bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = format!(
            "{} {} {}\r\n",
            self.http_version, self.status, self.reason_phrase
        );

        // Sort headers for deterministic output
        let mut headers: Vec<_> = self.headers.iter().collect();
        headers.sort_by(|a, b| a.0.cmp(b.0));

        for (key, value) in headers {
            result.push_str(&format!("{}: {}\r\n", key, value));
        }

        result.push_str("\r\n");
        result.into_bytes()
    }

    /// Combine headers and body into complete response
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut response = self.serialize();
        response.extend_from_slice(&self.body);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_from_str() {
        assert_eq!("GET".parse::<HttpMethod>(), Ok(HttpMethod::Get));
        assert_eq!("POST".parse::<HttpMethod>(), Ok(HttpMethod::Post));
        assert_eq!("PUT".parse::<HttpMethod>(), Ok(HttpMethod::Put));
        assert_eq!("DELETE".parse::<HttpMethod>(), Ok(HttpMethod::Delete));
        assert!("INVALID".parse::<HttpMethod>().is_err());
    }

    #[test]
    fn test_http_method_display() {
        assert_eq!(HttpMethod::Get.to_string(), "GET");
        assert_eq!(HttpMethod::Post.to_string(), "POST");
    }

    #[test]
    fn test_http_headers_case_insensitive() {
        let mut headers = HttpHeaders::new();
        headers.insert("Content-Type", "text/plain");
        
        assert_eq!(headers.get("content-type"), Some("text/plain"));
        assert_eq!(headers.get("Content-Type"), Some("text/plain"));
        assert_eq!(headers.get("CONTENT-TYPE"), Some("text/plain"));
    }

    #[test]
    fn test_http_response_ok() {
        let mut headers = HttpHeaders::new();
        headers.insert("Content-Type", "text/plain");
        let body = b"Hello".to_vec();
        let response = HttpResponse::ok(headers, body);
        
        assert_eq!(response.status, 200);
        assert_eq!(response.reason_phrase, "OK");
        assert_eq!(response.body, b"Hello");
    }

    #[test]
    fn test_http_response_formatting() {
        let mut headers = HttpHeaders::new();
        headers.insert("Content-Type", "text/plain");
        headers.insert("Content-Length", "5");

        let response = HttpResponse::ok(headers, b"Hello".to_vec());
        let bytes = response.to_bytes();

        // Assert exact serialization (headers are sorted alphabetically)
        let expected = b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\ncontent-type: text/plain\r\n\r\nHello";

        assert_eq!(
            bytes, expected,
            "Response does not match expected format. Got:\n{}",
            String::from_utf8_lossy(&bytes)
        );
    }
}

