// Integration test utilities
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use codecrafters_http_server::{Server, ServerConfig};

/// How long to wait for the server port to become available.
const PORT_READY_TIMEOUT: Duration = Duration::from_secs(5);

/// How often to check if the port is available.
const PORT_CHECK_INTERVAL: Duration = Duration::from_millis(50);

/// Timeout for socket read/write operations.
const SOCKET_TIMEOUT: Duration = Duration::from_secs(5);

/// Buffer size for reading HTTP responses.
const READ_BUFFER_SIZE: usize = 4096;

/// How long to wait before retrying on WouldBlock.
const WOULD_BLOCK_RETRY_INTERVAL: Duration = Duration::from_millis(10);

/// Wait for a port to become available within timeout
fn wait_for_port(port: u16) -> bool {
    let start = std::time::Instant::now();
    loop {
        match TcpStream::connect(format!("127.0.0.1:{}", port)) {
            Ok(_) => return true,
            Err(_) => {
                if start.elapsed() > PORT_READY_TIMEOUT {
                    return false;
                }
                thread::sleep(PORT_CHECK_INTERVAL);
            }
        }
    }
}

/// Test server that wraps the real Server for integration testing.
/// Provides convenience methods for sending test requests.
pub struct TestServer {
    server: Server,
}

impl TestServer {
    /// Create and start a new test server with optional file directory
    pub fn start(directory: Option<String>) -> Self {
        let config = ServerConfig::new(directory);
        let server =
            Server::start_with_dynamic_port(config).expect("Failed to start test server");

        // Wait for server to be ready
        wait_for_port(server.port());

        TestServer { server }
    }

    /// Get the server URL for making requests
    pub fn url(&self) -> String {
        format!("http://{}", self.server.addr())
    }

    /// Get the server port number
    pub fn port(&self) -> u16 {
        self.server.port()
    }

    /// Get the server address as "127.0.0.1:port"
    pub fn addr(&self) -> String {
        self.server.addr().to_string()
    }

    /// Send a raw HTTP request and receive the response as bytes.
    /// Reads until connection closes (use Connection: close header).
    pub fn send_request_bytes(&self, request: &str) -> Vec<u8> {
        let mut stream = self.connect();

        stream
            .write_all(request.as_bytes())
            .expect("Failed to write request");

        let mut response = Vec::new();
        let mut buffer = [0; READ_BUFFER_SIZE];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => response.extend_from_slice(&buffer[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(WOULD_BLOCK_RETRY_INTERVAL);
                }
                Err(e) => {
                    eprintln!("Read error: {}", e);
                    break;
                }
            }
        }

        response
    }

    /// Send a raw HTTP request and receive the response as a string
    pub fn send_request(&self, request: &str) -> String {
        self.send_requests(&[request]).join("\n")
    }

    /// Send multiple HTTP requests over a single persistent connection.
    /// Returns a response for each request. The last request should include
    /// `Connection: close` to properly terminate the connection.
    pub fn send_requests(&self, requests: &[&str]) -> Vec<String> {
        let stream = self.connect();
        let mut reader = BufReader::new(stream);

        let mut responses = Vec::with_capacity(requests.len());

        for request in requests {
            reader
                .get_mut()
                .write_all(request.as_bytes())
                .expect("Failed to write request");

            let response = Self::read_single_response(&mut reader);
            responses.push(response);
        }

        responses
    }

    fn connect(&self) -> TcpStream {
        let stream =
            TcpStream::connect(self.server.addr()).expect("Failed to connect to test server");
        stream
            .set_read_timeout(Some(SOCKET_TIMEOUT))
            .expect("Failed to set read timeout");
        stream
            .set_write_timeout(Some(SOCKET_TIMEOUT))
            .expect("Failed to set write timeout");
        stream
    }

    /// Read a single HTTP response by parsing headers for Content-Length
    fn read_single_response<R: BufRead>(reader: &mut R) -> String {
        // Read status line (e.g., "HTTP/1.1 200 OK\r\n")
        let mut status_line = String::new();
        reader
            .read_line(&mut status_line)
            .expect("Failed to read status line");

        // Read headers until empty line
        let mut headers = String::new();
        let mut content_length: usize = 0;
        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .expect("Failed to read header line");
            if line == "\r\n" {
                break;
            }
            if line.to_lowercase().starts_with("content-length:") {
                content_length = line
                    .split(':')
                    .nth(1)
                    .unwrap()
                    .trim()
                    .parse()
                    .expect("Invalid Content-Length");
            }
            headers.push_str(&line);
        }

        // Read body based on Content-Length
        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            reader.read_exact(&mut body).expect("Failed to read body");
        }

        format!(
            "{}{}\r\n{}",
            status_line,
            headers,
            String::from_utf8_lossy(&body)
        )
    }
}
