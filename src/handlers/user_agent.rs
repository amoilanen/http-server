use crate::http::{HttpHeaders, HttpRequest, HttpResponse};

/// Handle GET/POST request to "/user-agent"
pub fn handle_user_agent(request: &HttpRequest) -> HttpResponse {
    let user_agent = request
        .headers
        .get("User-Agent")
        .unwrap_or("Unknown");

    let body = user_agent.as_bytes().to_vec();
    let mut headers = HttpHeaders::new();

    headers.insert("Content-Type", "text/plain");
    headers.insert("Content-Length", body.len().to_string());

    HttpResponse::ok(headers, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::HttpMethod;

    #[test]
    fn test_handle_user_agent_present() {
        let mut headers = HttpHeaders::new();
        headers.insert("User-Agent", "Mozilla/5.0");
        
        let request = HttpRequest::new(
            HttpMethod::Get,
            "/user-agent".to_string(),
            "HTTP/1.1".to_string(),
            headers,
            Vec::new(),
        );
        
        let response = handle_user_agent(&request);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"Mozilla/5.0");
    }

    #[test]
    fn test_handle_user_agent_missing() {
        let request = HttpRequest::new(
            HttpMethod::Get,
            "/user-agent".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );
        
        let response = handle_user_agent(&request);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"Unknown");
    }
}

