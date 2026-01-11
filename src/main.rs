use std::io::Write;
use std::net::TcpListener;
use std::thread;

mod http;
mod compression;
mod config;
mod handlers;

use http::parse_request;
use config::parse_args;
use handlers::Router;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Logs from your program will appear here!");

    let server_config = parse_args()?;
    println!("Server configuration: {:?}", server_config);

    let listener = TcpListener::bind("127.0.0.1:4221")?;
    println!("Server listening on 127.0.0.1:4221");

    for stream in listener.incoming() {
        match stream {
            Ok(mut tcp_stream) => {
                let router = Router::new(server_config.clone());
                thread::spawn(move || {
                    println!("accepted new connection");
                    match handle_connection(&mut tcp_stream, &router) {
                        Ok(()) => println!("Handled request correctly"),
                        Err(e) => println!("Error while handling a request: {}", e),
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_connection(
    stream: &mut std::net::TcpStream,
    router: &handlers::Router,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = parse_request(stream)?;
    let response = router.handle(request);
    stream.write_all(&response.to_bytes())?;
    Ok(())
}
