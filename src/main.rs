use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;

use anyhow::Result;

mod args;
mod http;
mod compression;
mod config;
mod handlers;

use http::parse_request;
use config::parse_args;
use handlers::Router;

fn main() -> Result<()> {
    // Initialize logging with env_logger
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .init();

    let server_config = parse_args()?;
    log::info!("Server configuration: {:?}", server_config);

    let listener = TcpListener::bind("127.0.0.1:4221")?;
    log::info!("Server listening on 127.0.0.1:4221");

    for stream in listener.incoming() {
        match stream {
            Ok(mut tcp_stream) => {
                let router = Router::new(server_config.clone());
                thread::spawn(move || {
                    log::debug!("Accepted new connection");
                    match handle_connection(&mut tcp_stream, &router) {
                        Ok(()) => log::debug!("Handled request correctly"),
                        Err(e) => log::error!("Error while handling a request: {}", e),
                    }
                });
            }
            Err(e) => {
                log::error!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_connection(stream: &mut TcpStream, router: &Router) -> Result<()> {
    let request = parse_request(stream)?;
    let response = router.handle(request);
    stream.write_all(&response.to_bytes())?;
    Ok(())
}
