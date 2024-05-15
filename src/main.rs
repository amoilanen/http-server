use std::io::BufRead;
use std::io::Read;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::Write;
use std::io::BufReader;
use std::io::{ ErrorKind, Error };

use itertools::Itertools;

#[derive(Debug)]
struct HttpRequest {
    contents: String
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, std::io::Error> {
    const BUFFER_SIZE: usize = 1024;
    let mut reader = BufReader::new(stream);
    let mut contents = String::new();
    let mut buffer = [0; BUFFER_SIZE];
    let mut finished_reading_request = false;

    while !finished_reading_request {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            finished_reading_request = true;
        } else {
            contents.push_str(&String::from_utf8_lossy(&buffer[..bytes_read]));
            if bytes_read < BUFFER_SIZE {
                finished_reading_request = true;
            }
        }
    }
    Ok(HttpRequest { contents })
}

fn handle_request(mut stream: TcpStream) -> Result<(), std::io::Error> {
    let request = read_request(&mut stream)?;
    //println!("{:?}", request);
    let request_all_lines = request.contents.split("\r\n").collect::<Vec<&str>>();
    let request_line = request_all_lines.first().ok_or(Error::new(ErrorKind::Other, "Malformed HTTP request"))?;
    let request_line_parts = request_line.split_whitespace().collect_vec();
    //let http_method = request_line_parts.get(0).ok_or(Error::new(ErrorKind::Other, "Malformed HTTP request"))?;
    let request_target = *request_line_parts.get(1).ok_or(Error::new(ErrorKind::Other, "Malformed HTTP request"))?;
    match request_target {
        "/" => {
            let response = "HTTP/1.1 200 OK\r\n\r\n";
            stream.write_all(response.as_bytes())
        }
        _ => {
            let response = "HTTP/1.1 404 Not Found\r\n\r\n";
            stream.write_all(response.as_bytes())
        }
    }
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
