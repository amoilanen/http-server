use crate::http::{HttpHeaders, HttpRequest, HttpResponse};

/// Handle GET request to "/"
pub fn handle_root(_request: &HttpRequest) -> HttpResponse {
    HttpResponse::ok(HttpHeaders::new(), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::HttpMethod;

    #[test]
    fn test_handle_root() {
        let request = HttpRequest::new(
            HttpMethod::Get,
            "/".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );
        
        let response = handle_root(&request);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, Vec::new());
    }
}

