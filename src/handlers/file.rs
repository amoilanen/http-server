use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::http::{HttpHeaders, HttpMethod, HttpRequest, HttpResponse};
use crate::config::ServerConfig;

/// Handle GET/POST request to "/files/*"
pub fn handle_file(request: &HttpRequest, config: &ServerConfig) -> HttpResponse {
    let uri = &request.uri;
    let file_name = &uri["/files/".len()..];

    match &config.directory {
        Some(directory) => match request.method {
            HttpMethod::Get => handle_get_file(directory, file_name),
            HttpMethod::Post => handle_post_file(directory, file_name, request),
            _ => HttpResponse::not_found(),
        },
        None => HttpResponse::not_found(),
    }
}

/// Handle GET request to retrieve a file
fn handle_get_file(directory: &str, file_name: &str) -> HttpResponse {
    let file_path = format!("{}/{}", directory, file_name);
    
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
fn handle_post_file(directory: &str, file_name: &str, request: &HttpRequest) -> HttpResponse {
    let file_path = format!("{}/{}", directory, file_name);

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

    #[test]
    fn test_handle_file_no_directory() {
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
    }

    #[test]
    fn test_handle_file_not_found() {
        let config = ServerConfig::new(Some("/tmp".to_string()));
        let request = HttpRequest::new(
            HttpMethod::Get,
            "/files/nonexistent_file_xyz.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );
        
        let response = handle_file(&request, &config);
        assert_eq!(response.status, 404);
    }

    #[test]
    fn test_handle_post_file_creates_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();
        let config = ServerConfig::new(Some(temp_path.to_string()));
        
        let request = HttpRequest::new(
            HttpMethod::Post,
            "/files/test.txt".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            b"test content".to_vec(),
        );
        
        let response = handle_file(&request, &config);
        assert_eq!(response.status, 201);
        assert_eq!(response.body, b"Uploaded successfully");
    }
}

