use std::io::BufRead;
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
use std::fs::OpenOptions;
use std::path::Path;
use std::usize;
use flate2::write::GzEncoder;

use itertools::Itertools;

#[derive(Debug, PartialEq)]
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
    headers: HttpHeaders,
    body: Vec<u8>
}

struct RequestLine {
    method: HttpMethod,
    uri: String,
    http_version: String,
}

#[derive(Debug)]
struct HttpHeaders {
    name_value_pairs: Vec<(String, String)>
}

impl HttpHeaders {
    fn new(name_value_pairs: Vec<(String, String)>) -> HttpHeaders {
        HttpHeaders {
            name_value_pairs
        }
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.name_value_pairs.iter().find(|(header_name, _)| header_name == name).map(|(_, header_value)| header_value.as_str())
    }

    fn append(&mut self, name: String, value: String) {
        self.name_value_pairs.push((name, value));
    }

    fn empty() -> HttpHeaders {
        HttpHeaders::new(Vec::new())
    }
}

struct HttpResponse {
    http_version: String,
    status: u16,
    reason_phrase: String,
    headers: HttpHeaders,
    body: Vec<u8>
}

impl HttpResponse {

    fn ok_with_bytes(headers: HttpHeaders, body: Vec<u8>) -> HttpResponse {
        HttpResponse {
            http_version: String::from("HTTP/1.1"),
            status: 200,
            reason_phrase: String::from("OK"),
            headers,
            body
        }
    }

    fn ok(headers: HttpHeaders, body: &str) -> HttpResponse {
        HttpResponse {
            http_version: String::from("HTTP/1.1"),
            status: 200,
            reason_phrase: String::from("OK"),
            headers,
            body: body.as_bytes().to_vec()
        }
    }

    fn created(headers: HttpHeaders, body: &str) -> HttpResponse {
        HttpResponse {
            http_version: String::from("HTTP/1.1"),
            status: 201,
            reason_phrase: String::from("Created"),
            headers,
            body: body.as_bytes().to_vec()
        }
    }

    fn not_found() -> HttpResponse {
        HttpResponse {
            http_version: String::from("HTTP/1.1"),
            status: 404,
            reason_phrase: String::from("Not Found"),
            headers: HttpHeaders::empty(),
            body: Vec::new()
        }
    }

    fn format_status_line_and_headers(&self) -> String {
        let mut formatted_headers = String::new();
        for header in self.headers.name_value_pairs.iter() {
            formatted_headers.push_str(format!("{}: {}\r\n", header.0, header.1).as_str());
        }
        format!("{} {} {}\r\n{}\r\n", self.http_version.as_str(), self.status, self.reason_phrase, formatted_headers.as_str())
    }

    fn write_to(&self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
        stream.write_all(self.format_status_line_and_headers().as_bytes())?;
        stream.write_all(&self.body)
    }
}

fn parse_request_line(reader: &mut BufReader<&mut TcpStream>) -> Result<RequestLine, std::io::Error> {
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let request_line_parts: Vec<&str> = request_line.split_whitespace().collect();
    let method_input =  *request_line_parts.get(0)
        .ok_or(Error::new(ErrorKind::Other, format!("Malformed HTTP request: cannot parse HTTP method: '{}'", request_line)))?;
    let method = HttpMethod::from_str(method_input).map_err(|err| Error::new(ErrorKind::Other, format!("Malformed HTTP request: cannot parse HTTP method: '{}'", err)))?;
    let uri =  String::from(*request_line_parts.get(1)
        .ok_or(Error::new(ErrorKind::Other, format!("Malformed HTTP request: cannot parse request URI: '{}'", request_line)))?);
    let http_version =  String::from(*request_line_parts.get(2)
        .ok_or(Error::new(ErrorKind::Other, format!("Malformed HTTP request: cannot parse request HTTP version: '{}'", request_line)))?);
    Ok(RequestLine {
        method,
        uri,
        http_version
    })
}

fn parse_http_headers(reader: &mut BufReader<&mut TcpStream>) -> Result<HttpHeaders, std::io::Error> {
    let mut name_value_pairs: Vec<(String, String)> = Vec::new();
    let mut current_header_line = String::new();
    loop {
        match reader.read_line(&mut current_header_line)? {
            0 => break,
            _ => {
                if current_header_line == "\r\n" {
                    break;
                } else {
                    let header_parts = current_header_line
                        .split_once(":").ok_or(Error::new(ErrorKind::Other, format!("Malformed HTTP header: '{}'", current_header_line)))?;
                    let header = (String::from(header_parts.0.trim()), String::from(header_parts.1.trim()));
                    name_value_pairs.push(header);
                }
                current_header_line.clear();
            }
        }
    }
    Ok(HttpHeaders::new(name_value_pairs))
}

fn get_content_length_from_headers(http_headers: &HttpHeaders) -> Result<usize, std::io::Error> {
    let content_length_header_value = http_headers.name_value_pairs.iter()
        .find(|(header_name, _)| header_name == "Content-Length")
        .map(|(_, header_value)| header_value.as_str()).unwrap_or("0");
    let content_length = content_length_header_value.parse::<usize>()
        .or_else(|_| Err(Error::new(ErrorKind::Other, format!("Could not parse Content-Length header value '{}'", content_length_header_value))))?;
    Ok(content_length)
}

fn parse_request(stream: &mut TcpStream) -> Result<HttpRequest, std::io::Error> {
    let mut reader: BufReader<&mut TcpStream> = BufReader::new(stream);

    let request_line = parse_request_line(&mut reader)?;
    let http_headers = parse_http_headers(&mut reader)?;
    let content_length = get_content_length_from_headers(&http_headers)?;
    let mut body: Vec<u8> = vec![0; content_length];
    reader.read_exact(&mut body)?;

    Ok(HttpRequest {
        method: request_line.method,
        uri: request_line.uri,
        http_version: request_line.http_version,
        headers: http_headers,
        body
    })
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, std::io::Error> {
    parse_request(stream)
}

fn gzip_compress(bytes: Vec<u8>) -> Result<Vec<u8>,std::io::Error> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut encoder = GzEncoder::new(&mut buffer, flate2::Compression::default());
    encoder.write_all(&bytes)?;
    encoder.finish()?;
    Ok(buffer)
}

fn handle_request(mut stream: TcpStream, server_configuration: &ServerConfiguration) -> Result<(), std::io::Error> {
    let request = read_request(&mut stream)?;
    let uri = request.uri.as_str();
    if uri == "/" {
        let response = &HttpResponse::ok(HttpHeaders::empty(), "");
        response.write_to(&mut stream)
    } else if uri.starts_with("/echo/") {
        let str_uri_parameter =&uri["/echo/".len()..];
        let mut body = str_uri_parameter.as_bytes().to_vec();
        let mut headers = HttpHeaders::new(vec![
            (String::from("Content-Type"), String::from("text/plain"))
        ]);
        if let Some(accepted_encodings) = request.headers.get("Accept-Encoding") {
            let encodings: Vec<&str> = accepted_encodings.split(",").map(|encoding| encoding.trim()).collect();
            if encodings.iter().contains(&"gzip") {
                (&mut headers).append(String::from("Content-Encoding"), String::from("gzip"));
                body = gzip_compress(body)?
            }
        }
        (&mut headers).append( String::from("Content-Length"), body.len().to_string());
        let response = &HttpResponse::ok_with_bytes(headers, body);
        response.write_to(&mut stream)
    } else if uri == "/user-agent" {
        let user_agent_from_request_headers = if let Some(user_agent) = request.headers.name_value_pairs.iter().find(|header| header.0 == "User-Agent") {
            &user_agent.1
        } else {
            "Unknown"
        };
        let body = user_agent_from_request_headers;
        let headers = HttpHeaders::new(vec![
            (String::from("Content-Type"), String::from("text/plain")),
            (String::from("Content-Length"), body.len().to_string())
        ]);
        let response = &HttpResponse::ok(headers, body);
        response.write_to(&mut stream)
    } else if uri.starts_with("/files/") {
        match &server_configuration.directory {
            Some(directory) => {
                if request.method == HttpMethod::GET {
                  let file_name =&uri["/files/".len()..];
                  let file_path = directory.clone() + "/" + file_name;
                  if Path::new(&file_path).exists() {
                      let file_bytes: Vec<u8> = fs::read(file_path)?;
                      let headers = HttpHeaders::new(vec![
                          (String::from("Content-Type"), String::from("application/octet-stream")),
                          (String::from("Content-Length"), file_bytes.len().to_string())
                      ]);
                      let response = &HttpResponse::ok_with_bytes(headers, file_bytes);
                      response.write_to(&mut stream)
                  } else {
                      let response = &HttpResponse::not_found();
                      response.write_to(&mut stream)
                  }
                } else if request.method == HttpMethod::POST {
                    let file_name =&uri["/files/".len()..];
                    let file_path = directory.clone() + "/" + file_name;
                    let mut file = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(file_path)?;
                    println!("Length of request body = {}", request.body.len());
                    file.write_all(&request.body)?;
                    let body = "Uploaded successfully";
                    let headers = HttpHeaders::new(vec![
                        (String::from("Content-Type"), String::from("text/plain")),
                        (String::from("Content-Length"), body.len().to_string())
                    ]);
                    let response = &HttpResponse::created(headers, &body);
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
