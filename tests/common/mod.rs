// Integration test utilities
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

/// Find an available port for testing
#[allow(dead_code)]
pub fn find_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to port 0");
    let addr = listener.local_addr().expect("Failed to get local address");
    addr.port()
}

/// Wait for a port to become available
#[allow(dead_code)]
pub fn wait_for_port(port: u16, timeout_secs: u64) -> bool {
    let start = std::time::Instant::now();
    loop {
        match std::net::TcpStream::connect(format!("127.0.0.1:{}", port)) {
            Ok(_) => return true,
            Err(_) => {
                if start.elapsed() > Duration::from_secs(timeout_secs) {
                    return false;
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// Start a test server in a background thread
#[allow(dead_code)]
pub struct TestServer {
    port: u16,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

#[allow(dead_code)]
impl TestServer {
    pub fn new(directory: Option<String>) -> Self {
        let port = find_free_port();
        
        let thread_handle = thread::spawn(move || {
            let config = codecrafters_http_server::ServerConfig::new(directory);
            let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
                .expect("Failed to bind to test port");
            
            // Only accept a few connections for testing
            for stream in listener.incoming().take(10) {
                if let Ok(mut tcp_stream) = stream {
                    let router = codecrafters_http_server::Router::new(config.clone());
                    
                    if let Ok(request) = codecrafters_http_server::http::parse_request(&mut tcp_stream) {
                        let response = router.handle(request);
                        let _ = std::io::Write::write_all(&mut tcp_stream, &response.to_bytes());
                    }
                }
            }
        });
        
        // Wait for port to be available
        wait_for_port(port, 5);
        
        TestServer {
            port,
            thread_handle: Some(thread_handle),
        }
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // Server thread will exit when listener is dropped
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

