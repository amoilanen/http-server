// Library root - exposes public API for integration tests

pub mod args;
pub mod http;
pub mod compression;
pub mod config;
pub mod handlers;
pub mod server;

pub use http::{HttpMethod, HttpRequest, HttpResponse, HttpHeaders, parse_request};
pub use config::ServerConfig;
pub use handlers::Router;
pub use server::Server;

