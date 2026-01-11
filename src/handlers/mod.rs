use crate::http::{HttpRequest, HttpResponse};
use crate::config::ServerConfig;

pub mod echo;
pub mod user_agent;
pub mod file;
pub mod root;

pub use echo::handle_echo;
pub use user_agent::handle_user_agent;
pub use file::handle_file;
pub use root::handle_root;

/// Router that dispatches requests to appropriate handlers
pub struct Router {
    config: ServerConfig,
}

impl Router {
    pub fn new(config: ServerConfig) -> Self {
        Router { config }
    }

    /// Route a request to the appropriate handler
    pub fn handle(&self, request: HttpRequest) -> HttpResponse {
        let uri = &request.uri;

        if uri == "/" {
            handle_root(&request)
        } else if uri.starts_with("/echo/") {
            handle_echo(&request)
        } else if uri == "/user-agent" {
            handle_user_agent(&request)
        } else if uri.starts_with("/files/") {
            handle_file(&request, &self.config)
        } else {
            HttpResponse::not_found()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{HttpMethod, HttpHeaders};

    #[test]
    fn test_router_root() {
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
    }

    #[test]
    fn test_router_not_found() {
        let config = ServerConfig::new(None);
        let router = Router::new(config);
        
        let request = HttpRequest::new(
            HttpMethod::Get,
            "/unknown".to_string(),
            "HTTP/1.1".to_string(),
            HttpHeaders::new(),
            Vec::new(),
        );
        
        let response = router.handle(request);
        assert_eq!(response.status, 404);
    }
}

