use crate::http::{HttpHeaders, HttpRequest, HttpResponse};
use crate::compression;

/// Handle GET/POST request to "/echo/*"
pub fn handle_echo(request: &HttpRequest) -> HttpResponse {
    let uri = &request.uri;
    let echo_text = &uri["/echo/".len()..];
    let mut body = echo_text.as_bytes().to_vec();
    let mut headers = HttpHeaders::new();

    headers.insert("Content-Type", "text/plain");

    // Check for gzip encoding acceptance
    if let Some(accept_encoding) = request.headers.get("Accept-Encoding") {
        let encodings: Vec<&str> = accept_encoding.split(',').map(|e| e.trim()).collect();
        if encodings.iter().any(|&e| e == "gzip") {
            if let Ok(compressed) = compression::gzip_encode(&body) {
                body = compressed;
                headers.insert("Content-Encoding", "gzip");
            }
        }
    }

    headers.insert("Content-Length", body.len().to_string());
    HttpResponse::ok(headers, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::HttpMethod;

    #[test]
    fn test_handle_echo_simple() {
        let request = HttpRequest::new(
            HttpMethod::Get,
            "/echo/hello".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );
        
        let response = handle_echo(&request);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"hello");
        assert_eq!(response.headers.get("Content-Type"), Some("text/plain"));
    }

    #[test]
    fn test_handle_echo_with_gzip() {
        let mut headers = HttpHeaders::new();
        headers.insert("Accept-Encoding", "gzip");
        
        let request = HttpRequest::new(
            HttpMethod::Get,
            "/echo/hello".to_string(),
            "HTTP/1.1".to_string(),
            headers,
            Vec::new(),
        );
        
        let response = handle_echo(&request);
        assert_eq!(response.status, 200);
        assert_eq!(response.headers.get("Content-Encoding"), Some("gzip"));
        assert_ne!(response.body, b"hello");
    }

    #[test]
    fn test_handle_echo_empty_path() {
        let request = HttpRequest::new(
            HttpMethod::Get,
            "/echo/".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );
        
        let response = handle_echo(&request);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"");
    }
}

