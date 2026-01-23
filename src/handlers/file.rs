use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::http::{HttpHeaders, HttpMethod, HttpRequest, HttpResponse};
use crate::config::ServerConfig;

/// Handle GET/POST request to "/files/*"
pub fn handle_file(request: &HttpRequest, config: &ServerConfig) -> HttpResponse {
    let uri = &request.uri;
    let file_path = &uri["/files/".len()..];

    match &config.directory {
        Some(directory) => match request.method {
            HttpMethod::Get => handle_get_file(directory, file_path),
            HttpMethod::Post => handle_post_file(directory, file_path, request),
            _ => HttpResponse::not_found(),
        },
        None => HttpResponse::not_found(),
    }
}

/// Handle GET request to retrieve a file
fn handle_get_file(directory: &str, file_path: &str) -> HttpResponse {
    let file_path = format!("{}/{}", directory, file_path);
    
    if !Path::new(&file_path).exists() {
        return HttpResponse::not_found();
    }

    match fs::read(&file_path) {
        Ok(file_bytes) => {
            let mut headers = HttpHeaders::new();
            headers.insert("Content-Type", "application/octet-stream");
            headers.insert("Content-Length", file_bytes.len().to_string());
            HttpResponse::ok(headers, file_bytes)
        }
        Err(_) => HttpResponse::not_found(),
    }
}

/// Handle POST request to upload/create a file
fn handle_post_file(directory: &str, file_path: &str, request: &HttpRequest) -> HttpResponse {
    let file_path = format!("{}/{}", directory, file_path);

    match OpenOptions::new()
        .create(true)
        .write(true)
        .open(&file_path)
    {
        Ok(mut file) => {
            if let Err(_) = file.write_all(&request.body) {
                return HttpResponse::not_found();
            }

            let body = b"Uploaded successfully".to_vec();
            let mut headers = HttpHeaders::new();
            headers.insert("Content-Type", "text/plain");
            headers.insert("Content-Length", body.len().to_string());
            
            HttpResponse::created(headers, body)
        }
        Err(_) => HttpResponse::not_found(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use anyhow::Result;

    #[test]
    fn test_handle_file_no_server_directory() -> Result<()> {
        let config = ServerConfig::new(None);
        let request = HttpRequest::new(
            HttpMethod::Get,
            "/files/test.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );

        let response = handle_file(&request, &config);
        assert_eq!(response.status, 404);
        Ok(())
    }

    #[test]
    fn test_handle_get_file_not_found() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_str().ok_or_else(|| anyhow::anyhow!("Failed to get temp path"))?;
        let config = ServerConfig::new(Some(temp_path.to_string()));


        fs::create_dir_all(format!("{}/my-dir", temp_path))?;

        let mut request = HttpRequest::new(
            HttpMethod::Get,
            "/files/nonexistent.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );

        let mut response = handle_file(&request, &config);
        assert_eq!(response.status, 404);
        request = HttpRequest::new(
            HttpMethod::Get,
            "/files/my-dir/nonexistent.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );
        response = handle_file(&request, &config);
        assert_eq!(response.status, 404);

        Ok(())
    }

    #[test]
    fn test_handle_post_file_creates_and_retrieves() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_str().ok_or_else(|| anyhow::anyhow!("Failed to get temp path"))?;
        let config = ServerConfig::new(Some(temp_path.to_string()));

        let content = b"test content".to_vec();
        let post_request = HttpRequest::new(
            HttpMethod::Post,
            "/files/test.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            content.clone(),
        );

        let post_response = handle_file(&post_request, &config);
        assert_eq!(post_response.status, 201, "Expected 201 Created status");
        assert_eq!(post_response.body, b"Uploaded successfully");

        let get_request = HttpRequest::new(
            HttpMethod::Get,
            "/files/test.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );
        
        let get_response = handle_file(&get_request, &config);
        assert_eq!(get_response.status, 200, "Expected 200 OK status");
        assert_eq!(get_response.body, content, "Response body should match uploaded content");
        
        // Verify GET response headers
        assert_eq!(
            get_response.headers.get("Content-Type"),
            Some("application/octet-stream"),
            "Content-Type should be set"
        );
        assert_eq!(
            get_response.headers.get("Content-Length"),
            Some(content.len().to_string().as_str()),
            "Content-Length should match file size"
        );
        Ok(())
    }

    #[test]
    fn test_handle_post_file_with_various_content() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_str().ok_or_else(|| anyhow::anyhow!("Failed to get temp path"))?;
        let config = ServerConfig::new(Some(temp_path.to_string()));
        
        // Test with empty content
        let empty_request = HttpRequest::new(
            HttpMethod::Post,
            "/files/empty.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );
        
        let empty_response = handle_file(&empty_request, &config);
        assert_eq!(empty_response.status, 201);
        
        let empty_file = fs::read(format!("{}/empty.txt", temp_path))?;
        assert_eq!(empty_file.len(), 0, "Empty file should be created");
        
        // Test with binary content
        let binary_content = vec![0u8, 1, 2, 255, 254, 253];
        let binary_request = HttpRequest::new(
            HttpMethod::Post,
            "/files/binary.bin".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            binary_content.clone(),
        );
        
        let binary_response = handle_file(&binary_request, &config);
        assert_eq!(binary_response.status, 201);
        
        let binary_file = fs::read(format!("{}/binary.bin", temp_path))?;
        assert_eq!(binary_file, binary_content, "Binary content should be preserved");
        Ok(())
    }

    #[test]
    fn test_handle_post_file_overwrites_existing() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_str().ok_or_else(|| anyhow::anyhow!("Failed to get temp path"))?;
        let config = ServerConfig::new(Some(temp_path.to_string()));
        
        let file_path = format!("{}/overwrite.txt", temp_path);
        
        // Create initial file
        fs::write(&file_path, b"old content")?;
        
        // Upload new content
        let new_content = b"new content overwrite".to_vec();
        let request = HttpRequest::new(
            HttpMethod::Post,
            "/files/overwrite.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            new_content.clone(),
        );
        
        let response = handle_file(&request, &config);
        assert_eq!(response.status, 201);
        
        let file_contents = fs::read(&file_path)?;
        assert_eq!(file_contents, new_content, "File should be overwritten");
        Ok(())
    }

    #[test]
    fn test_handle_file_with_nested_directories() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_str().ok_or_else(|| anyhow::anyhow!("Failed to get temp path"))?;
        let config = ServerConfig::new(Some(temp_path.to_string()));
        
        // Test POST to single subdirectory
        fs::create_dir(format!("{}/subdir", temp_path))?;
        let content1 = b"file in subdirectory".to_vec();
        let request1 = HttpRequest::new(
            HttpMethod::Post,
            "/files/subdir/myfile.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            content1.clone(),
        );
        
        let response1 = handle_file(&request1, &config);
        assert_eq!(response1.status, 201);
        
        let file1 = fs::read(format!("{}/subdir/myfile.txt", temp_path))?;
        assert_eq!(file1, content1);
        
        // Test POST to deeply nested directories (pre-created)
        fs::create_dir_all(format!("{}/a/b/c/d", temp_path))?;
        let content2 = b"deep file".to_vec();
        let request2 = HttpRequest::new(
            HttpMethod::Post,
            "/files/a/b/c/d/deepfile.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            content2.clone(),
        );
        
        let response2 = handle_file(&request2, &config);
        assert_eq!(response2.status, 201);
        
        let file2 = fs::read(format!("{}/a/b/c/d/deepfile.txt", temp_path))?;
        assert_eq!(file2, content2);
        
        // Test GET from nested directory
        let get_request = HttpRequest::new(
            HttpMethod::Get,
            "/files/subdir/myfile.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );
        
        let get_response = handle_file(&get_request, &config);
        assert_eq!(get_response.status, 200);
        assert_eq!(get_response.body, content1);
        Ok(())
    }

}

