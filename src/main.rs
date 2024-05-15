use std::net::TcpListener;
use std::net::TcpStream;
use std::io::Write;

fn handle_request(mut stream: TcpStream) -> Result<(), std::io::Error> {
    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response.as_bytes())
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                match handle_request(_stream) {
                    Ok(_) =>
                        println!("Handled request correctly"),
                    Err(e) =>
                        println!("Error while handling a request: {}", e)
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
