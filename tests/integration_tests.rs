mod common;

use common::TestServer;

// ============================================================================
// E2E Integration Tests - Testing via HTTP socket level
// ============================================================================

#[test]
fn test_basic_endpoints() {
    let server = TestServer::start(None);

    // Test root endpoint
    let response = server.send_request("GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK"), "Root endpoint failed. Got: {}", response);
    assert!(response.contains("\r\n\r\n"), "Response should have proper HTTP format");

    // Test unknown path returns 404
    let response = server.send_request("GET /unknown/path HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 404"), "404 handling failed. Got: {}", response);
}

#[test]
fn test_echo_endpoint_variants() {
    let server = TestServer::start(None);

    // Simple echo
    let response = server.send_request("GET /echo/hello-world HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.to_lowercase().contains("content-type: text/plain"));
    assert!(response.contains("hello-world"));

    // Echo with path segments
    let response = server.send_request("GET /echo/path/with/slashes HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains("path/with/slashes"));

    // Echo with special characters
    let response = server.send_request("GET /echo/hello%40world%23test HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains("hello%40world%23test"));
}

#[test]
fn test_echo_compression() {
    let server = TestServer::start(None);

    // Without Accept-Encoding: no compression
    let response = server.send_request("GET /echo/test-data HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(!response.to_lowercase().contains("content-encoding"));
    assert!(response.contains("test-data"));

    // With Accept-Encoding: gzip - response is compressed
    let response_bytes = server.send_request_bytes("GET /echo/test-data HTTP/1.1\r\nHost: localhost\r\nAccept-Encoding: gzip\r\nConnection: close\r\n\r\n");
    let response_str = String::from_utf8_lossy(&response_bytes);
    assert!(response_str.starts_with("HTTP/1.1 200 OK"));
    assert!(response_str.to_lowercase().contains("content-encoding: gzip"));

    // With unsupported encoding: no compression
    let response = server.send_request("GET /echo/plain HTTP/1.1\r\nHost: localhost\r\nAccept-Encoding: deflate\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(!response.to_lowercase().contains("content-encoding"));
    assert!(response.contains("plain"));
}

#[test]
fn test_user_agent_header() {
    let server = TestServer::start(None);

    // With User-Agent header
    let response = server.send_request("GET /user-agent HTTP/1.1\r\nHost: localhost\r\nUser-Agent: TestClient/1.0\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains("TestClient/1.0"));

    // Without User-Agent header
    let response = server.send_request("GET /user-agent HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains("Unknown"));
}

#[test]
fn test_file_operations_for_server_without_configured_directory() {
    let server = TestServer::start(None);

    // POST without directory should fail
    let post_request = "POST /files/test.txt HTTP/1.1\r\nHost: localhost\r\nContent-Length: 7\r\nConnection: close\r\n\r\ncontent";
    let response = server.send_request(post_request);
    assert!(response.starts_with("HTTP/1.1 404"));

    // GET without directory should fail
    let response = server.send_request("GET /files/test.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 404"));
}

#[test]
fn test_file_operations_with_directory() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap().to_string();
    let server = TestServer::start(Some(temp_path.clone()));

    // Create a file via POST
    let file_content = "test file content";
    let post_request = format!(
        "POST /files/testfile.txt HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        file_content.len(),
        file_content
    );
    let response = server.send_request(&post_request);
    assert!(response.starts_with("HTTP/1.1 201"), "POST should return 201. Got: {}", response);

    // Verify file was created on disk
    let file_path = format!("{}/testfile.txt", temp_path);
    assert!(std::path::Path::new(&file_path).exists(), "File should exist on disk");

    // Retrieve the file via GET
    let get_request = "GET /files/testfile.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let response = server.send_request(get_request);
    assert!(response.starts_with("HTTP/1.1 200"), "GET should return 200. Got: {}", response);
    assert!(response.contains("test file content"), "Response should contain file content");

    // Verify GET to nonexistent file returns 404
    let response = server.send_request("GET /files/nonexistent.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 404"), "Nonexistent file should return 404");

    // Create another file to verify multiple files work
    let file_content2 = "another file";
    let post_request2 = format!(
        "POST /files/file2.txt HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        file_content2.len(),
        file_content2
    );
    let response = server.send_request(&post_request2);
    assert!(response.starts_with("HTTP/1.1 201"));

    let response = server.send_request("GET /files/file2.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    assert!(response.contains("another file"));
}

#[test]
fn test_multiple_sequential_requests() {
    let server = TestServer::start(None);

    // Send multiple requests sequentially to verify server handles them all
    for i in 1..=5 {
        let text = format!("request{}", i);
        let request = format!(
            "GET /echo/{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            text
        );
        let response = server.send_request(&request);
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains(&text), "Request {} should echo '{}'. Got: {}", i, text, response);
    }
}

#[test]
fn test_http_response_format() {
    let server = TestServer::start(None);

    // Verify proper HTTP response format with headers
    let response = server.send_request("GET /echo/test HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");

    // Must start with status line
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));

    // Must have headers section separated from body
    assert!(response.contains("\r\n\r\n"), "Response must have proper header/body separation");

    // Must have Content-Type header (case-insensitive check)
    assert!(response.to_lowercase().contains("content-type:"));

    // Must have Content-Length header
    assert!(response.to_lowercase().contains("content-length:"));
}
