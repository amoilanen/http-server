use std::io::Read;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::Write;
use std::io::BufReader;
use std::io::{ ErrorKind, Error };
use std::str::FromStr;
use std::thread;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug)]
enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE
}

impl HttpMethod {
    fn as_str(&self) -> &str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE"
        }
    }
}

impl FromStr for HttpMethod {
    type Err = &'static str;
  
    fn from_str(s: &str) -> Result<Self, Self::Err> {
      match s.to_uppercase().as_str() {
        "GET" => Ok(HttpMethod::GET),
        "POST" => Ok(HttpMethod::POST),
        "PUT" => Ok(HttpMethod::PUT),
        "DELETE" => Ok(HttpMethod::DELETE),
        _ => Err("Unknown HTTP method"),
      }
    }
  }

#[derive(Debug)]
struct HttpRequest {
    method: HttpMethod,
    uri: String,
    http_version: String,
    headers: Vec<(String, String)>,
    body: String
}

struct HttpResponse {
    http_version: String,
    status: u16,
    reason_phrase: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>
}

impl HttpResponse {

    fn ok_with_bytes(headers: Vec<(String, String)>, body: Vec<u8>) -> HttpResponse {
        HttpResponse {
            http_version: String::from("HTTP/1.1"),
            status: 200,
            reason_phrase: String::from("OK"),
            headers: headers,
            body: body
        }
    }

    fn ok(headers: Vec<(String, String)>, body: &str) -> HttpResponse {
        HttpResponse {
            http_version: String::from("HTTP/1.1"),
            status: 200,
            reason_phrase: String::from("OK"),
            headers: headers,
            body: body.as_bytes().to_vec()
        }
    }

    fn not_found() -> HttpResponse {
        HttpResponse {
            http_version: String::from("HTTP/1.1"),
            status: 404,
            reason_phrase: String::from("Not Found"),
            headers: Vec::new(),
            body: Vec::new()
        }
    }

    fn status_line_and_headers(&self) -> String {
        let mut formatted_headers = String::new();
        for header in self.headers.iter() {
            formatted_headers.push_str(format!("{}: {}\r\n", header.0, header.1).as_str());
        }
        format!("{} {} {}\r\n{}\r\n", self.http_version.as_str(), self.status, self.reason_phrase, formatted_headers.as_str())
    }

    fn write_to(&self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
        stream.write_all(self.status_line_and_headers().as_bytes())?;
        stream.write_all(&self.body)
    }
}

fn read_from(stream: &mut TcpStream) -> Result<String, std::io::Error> {
    const BUFFER_SIZE: usize = 1024;
    let mut reader = BufReader::new(stream);
    let mut contents = String::new();
    let mut buffer = [0; BUFFER_SIZE];
    let mut finished_reading = false;

    while !finished_reading {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            finished_reading = true;
        } else {
            contents.push_str(&String::from_utf8_lossy(&buffer[..bytes_read]));
            if bytes_read < BUFFER_SIZE {
                finished_reading = true;
            }
        }
    }
    Ok(contents)
}

fn parse_request(input: &str) -> Result<HttpRequest, std::io::Error> {
    let lines = input.split("\r\n").collect::<Vec<&str>>();
    let request_line = lines.first()
        .ok_or(Error::new(ErrorKind::Other, format!("Malformed HTTP request: cannot find request line '{}'", input)))?;
    let request_line_parts: Vec<&str> = request_line.split_whitespace().collect();
    let method_input =  *request_line_parts.get(0)
        .ok_or(Error::new(ErrorKind::Other, format!("Malformed HTTP request: cannot parse HTTP method: '{}'", request_line)))?;
    let method = HttpMethod::from_str(method_input).map_err(|err| Error::new(ErrorKind::Other, format!("Malformed HTTP request: cannot parse HTTP method: '{}'", err)))?;
    let uri =  String::from(*request_line_parts.get(1)
        .ok_or(Error::new(ErrorKind::Other, format!("Malformed HTTP request: cannot parse request URI: '{}'", request_line)))?);
    let http_version =  String::from(*request_line_parts.get(1)
        .ok_or(Error::new(ErrorKind::Other, format!("Malformed HTTP request: cannot parse request URI: '{}'", request_line)))?);
    let mut headers: Vec<(String, String)> = Vec::new();
    for header_line in lines.iter().skip(1).take_while(|line| !line.is_empty()) {
        let header_parts = header_line
          .split_once(":").ok_or(Error::new(ErrorKind::Other, format!("Malformed HTTP header: '{}'", header_line)))?;
        let header = (String::from(header_parts.0.trim()), String::from(header_parts.1.trim()));
        headers.push(header);
    }
    let mut body = String::new();
    for body_line in lines.iter().skip(1 + headers.len()) {
        body.push_str(body_line);
        body.push('\n');
    }
    Ok(HttpRequest {
        method,
        uri,
        http_version,
        headers,
        body
    })
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, std::io::Error> {
    let contents = read_from(stream)?;
    parse_request(&contents)
}

fn handle_request(mut stream: TcpStream, server_configuration: &ServerConfiguration) -> Result<(), std::io::Error> {
    let request = read_request(&mut stream)?;
    let uri = request.uri.as_str();
    if uri == "/" {
        let response = &HttpResponse::ok(Vec::new(), "");
        response.write_to(&mut stream)
    } else if uri.starts_with("/echo/") {
        let str_uri_parameter =&uri["/echo/".len()..];
        let body = str_uri_parameter;
        let headers = vec![
            (String::from("Content-Type"), String::from("text/plain")),
            (String::from("Content-Length"), body.len().to_string())
        ];
        let response = &HttpResponse::ok(headers, body);
        response.write_to(&mut stream)
    } else if uri == "/user-agent" {
        let user_agent_from_request_headers = if let Some(user_agent) = request.headers.iter().find(|header| header.0 == "User-Agent") {
            &user_agent.1
        } else {
            "Unknown"
        };
        let body = user_agent_from_request_headers;
        let headers = vec![
            (String::from("Content-Type"), String::from("text/plain")),
            (String::from("Content-Length"), body.len().to_string())
        ];
        let response = &HttpResponse::ok(headers, body);
        response.write_to(&mut stream)
    } else if uri.starts_with("/files/") {
        match &server_configuration.directory {
            Some(directory) => {
                let file_name =&uri["/files/".len()..];
                let file_path = directory.clone() + "/" + file_name;
                if Path::new(&file_path).exists() {
                    let file_bytes: Vec<u8> = fs::read(file_path)?;
                    let headers = vec![
                        (String::from("Content-Type"), String::from("application/octet-stream")),
                        (String::from("Content-Length"), file_bytes.len().to_string())
                    ];
                    let response = &HttpResponse::ok_with_bytes(headers, file_bytes);
                    response.write_to(&mut stream)
                } else {
                    let response = &HttpResponse::not_found();
                    response.write_to(&mut stream)
                }
            }
            None => {
                let response = &HttpResponse::not_found();
                response.write_to(&mut stream)
            }
        }
    } else {
        let response = &HttpResponse::not_found();
        response.write_to(&mut stream)
    }
}

#[derive(Debug, Clone)]
struct ServerConfiguration {
    directory: Option<String>
}

fn parse_args() -> Result<ServerConfiguration, std::io::Error> {
    let mut directory: Option<String> = None;
    let args = env::args().collect::<Vec<String>>();
    for (idx, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "-d" | "--directory" => directory = args.get(idx + 1).map(|s| String::from(s)),
            _ => {},
          }
    }
    Ok(ServerConfiguration { directory })
}

fn main() -> Result<(), std::io::Error> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    let server_configuration = parse_args()?;

    println!("Server configuration: {:?}", server_configuration);
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                let per_thread_server_configuration = server_configuration.clone();
                thread::spawn(move || {
                    println!("accepted new connection");
                    match handle_request(_stream, &per_thread_server_configuration) {
                        Ok(_) =>
                            println!("Handled request correctly"),
                        Err(e) =>
                            println!("Error while handling a request: {}", e)
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
