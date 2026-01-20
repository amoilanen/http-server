mod common;

use codecrafters_http_server::{http::HttpMethod, HttpRequest, HttpHeaders};
use codecrafters_http_server::ServerConfig;
use codecrafters_http_server::Router;

// ============================================================================
// Integration Tests - Testing the complete request/response cycle
// ============================================================================

#[test]
fn test_root_endpoint() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 200);
    assert_eq!(response.reason_phrase, "OK");
    assert_eq!(response.body, Vec::new());
}

#[test]
fn test_echo_endpoint() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/echo/hello-world".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"hello-world");
    assert_eq!(response.headers.get("Content-Type"), Some("text/plain"));
}

#[test]
fn test_echo_endpoint_with_gzip() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let mut headers = HttpHeaders::new();
    headers.insert("Accept-Encoding", "gzip");

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/echo/test-data".to_string(),
        "HTTP/1.1".to_string(),
        headers,
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 200);
    assert_eq!(response.headers.get("Content-Encoding"), Some("gzip"));
    // Compressed data should be different
    assert_ne!(response.body, b"test-data");
    // But should have gzip magic number
    assert_eq!(response.body[0], 0x1f);
    assert_eq!(response.body[1], 0x8b);
}

#[test]
fn test_echo_endpoint_gzip_ignored_when_not_accepted() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let mut headers = HttpHeaders::new();
    headers.insert("Accept-Encoding", "deflate");

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/echo/plain".to_string(),
        "HTTP/1.1".to_string(),
        headers,
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"plain");
    assert_eq!(response.headers.get("Content-Encoding"), None);
}

#[test]
fn test_user_agent_endpoint_present() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let mut headers = HttpHeaders::new();
    headers.insert("User-Agent", "Mozilla/5.0");

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/user-agent".to_string(),
        "HTTP/1.1".to_string(),
        headers,
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"Mozilla/5.0");
    assert_eq!(response.headers.get("Content-Type"), Some("text/plain"));
}

#[test]
fn test_user_agent_endpoint_missing() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/user-agent".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"Unknown");
}

#[test]
fn test_file_get_without_directory() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/files/test.txt".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 404);
}

#[test]
fn test_file_post_without_directory() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Post,
        "/files/test.txt".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        b"content".to_vec(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 404);
}

#[test]
fn test_file_operations_with_directory() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();
    let config = ServerConfig::new(Some(temp_path.to_string()));
    let router = Router::new(config);

    // First, POST a file
    let file_content = b"test file content";
    let mut post_headers = HttpHeaders::new();
    post_headers.insert("Content-Length", file_content.len().to_string());

    let post_request = HttpRequest::new(
        HttpMethod::Post,
        "/files/myfile.txt".to_string(),
        "HTTP/1.1".to_string(),
        post_headers,
        file_content.to_vec(),
    );

    let post_response = router.handle(post_request);

    assert_eq!(post_response.status, 201);
    assert_eq!(post_response.reason_phrase, "Created");
    assert_eq!(post_response.body, b"Uploaded successfully");

    // Verify file was created
    let file_path = format!("{}/myfile.txt", temp_path);
    assert!(std::path::Path::new(&file_path).exists());

    // Now, GET the file
    let router2 = Router::new(ServerConfig::new(Some(temp_path.to_string())));
    let get_request = HttpRequest::new(
        HttpMethod::Get,
        "/files/myfile.txt".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let get_response = router2.handle(get_request);

    assert_eq!(get_response.status, 200);
    assert_eq!(get_response.body, file_content);
    assert_eq!(
        get_response.headers.get("Content-Type"),
        Some("application/octet-stream")
    );
}

#[test]
fn test_file_get_nonexistent() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();
    let config = ServerConfig::new(Some(temp_path.to_string()));
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/files/nonexistent.txt".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 404);
}

#[test]
fn test_not_found_route() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/unknown/path".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 404);
    assert_eq!(response.reason_phrase, "Not Found");
}

#[test]
fn test_unsupported_method() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Delete,
        "/files/test.txt".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 404);
}

#[test]
fn test_response_headers_included() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/echo/test".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    // Check response can be formatted properly
    let formatted = response.serialize();
    let formatted_str = String::from_utf8_lossy(&formatted);

    assert!(formatted_str.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(formatted_str.to_lowercase().contains("content-type:"));
    assert!(formatted_str.to_lowercase().contains("content-length:"));
    assert!(formatted_str.ends_with("\r\n"));
}

#[test]
fn test_echo_multiple_segments() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/echo/path/with/slashes".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"path/with/slashes");
}

#[test]
fn test_echo_with_special_characters() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let special_text = "hello@world#test$value";
    let request = HttpRequest::new(
        HttpMethod::Get,
        format!("/echo/{}", special_text),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    assert_eq!(response.status, 200);
    assert_eq!(response.body, special_text.as_bytes());
}

#[test]
fn test_content_length_header_accuracy() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    let request = HttpRequest::new(
        HttpMethod::Get,
        "/echo/hello".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );

    let response = router.handle(request);

    let content_length = response
        .headers
        .get("Content-Length")
        .and_then(|cl| cl.parse::<usize>().ok());

    assert_eq!(content_length, Some(5)); // "hello" is 5 bytes
    assert_eq!(response.body.len(), 5);
}

#[test]
fn test_multiple_requests_same_router() {
    let config = ServerConfig::new(None);
    let router = Router::new(config);

    // First request
    let req1 = HttpRequest::new(
        HttpMethod::Get,
        "/echo/first".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );
    let resp1 = router.handle(req1);
    assert_eq!(resp1.body, b"first");

    // Second request
    let req2 = HttpRequest::new(
        HttpMethod::Get,
        "/echo/second".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );
    let resp2 = router.handle(req2);
    assert_eq!(resp2.body, b"second");

    // Third request
    let req3 = HttpRequest::new(
        HttpMethod::Get,
        "/user-agent".to_string(),
        "HTTP/1.1".to_string(),
        HttpHeaders::new(),
        Vec::new(),
    );
    let resp3 = router.handle(req3);
    assert_eq!(resp3.body, b"Unknown");
}

