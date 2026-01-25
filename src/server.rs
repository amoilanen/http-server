//! HTTP Server module providing a reusable, stoppable server.

use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::config::ServerConfig;
use crate::handlers::Router;
use crate::http::parse_request;

/// How long to wait for in-flight requests to complete during shutdown.
const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_millis(50);

/// How often to poll for new connections in non-blocking accept loop.
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(10);

const PERSISTENT_CONNECTION_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// A running HTTP server that can be gracefully shut down.
pub struct Server {
    /// The address the server is bound to
    addr: SocketAddr,
    /// Shutdown flag shared with the server thread
    shutdown_flag: Arc<AtomicBool>,
    /// Handle to the server thread (None after shutdown)
    thread_handle: Option<JoinHandle<()>>,
}

impl Server {
    /// Start a new server bound to the given address.
    ///
    /// # Arguments
    /// * `addr` - Address to bind to (e.g., "127.0.0.1:4221" or "0.0.0.0:8080")
    /// * `config` - Server configuration (directory settings, etc.)
    ///
    /// # Returns
    /// A running Server instance, or an error if binding fails.
    pub fn start<A: ToSocketAddrs>(addr: A, config: ServerConfig) -> Result<Self> {
        let listener = TcpListener::bind(&addr).context("Failed to bind to address")?;

        let local_addr = listener
            .local_addr()
            .context("Failed to get local address")?;

        listener
            .set_nonblocking(true)
            .context("Failed to set non-blocking mode")?;

        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let shutdown_clone = Arc::clone(&shutdown_flag);
        let thread_handle = thread::spawn(move || {
            Self::run_accept_loop(listener, config, shutdown_clone);
        });

        log::info!("Server listening on {}", local_addr);

        Ok(Server {
            addr: local_addr,
            shutdown_flag,
            thread_handle: Some(thread_handle),
        })
    }

    /// Start a server that binds to an OS-assigned free port.
    ///
    /// Useful for testing when you need dynamic port allocation.
    pub fn start_with_dynamic_port(config: ServerConfig) -> Result<Self> {
        Self::start("127.0.0.1:0", config)
    }

    /// Get the address the server is bound to.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get the port number the server is listening on.
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Initiate graceful shutdown and wait for the server to stop.
    ///
    /// This sets the shutdown flag and waits for the accept loop to exit.
    /// Safe to call multiple times.
    pub fn shutdown(&mut self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            thread::sleep(SHUTDOWN_GRACE_PERIOD);
            let _ = handle.join();
        }
    }

    /// Check if the server is still running.
    pub fn is_running(&self) -> bool {
        !self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// The main accept loop - runs in a separate thread.
    fn run_accept_loop(
        listener: TcpListener,
        config: ServerConfig,
        shutdown_flag: Arc<AtomicBool>,
    ) {
        loop {
            // Check shutdown flag
            if shutdown_flag.load(Ordering::SeqCst) {
                log::debug!("Server shutdown requested");
                break;
            }

            match listener.accept() {
                Ok((stream, peer_addr)) => {
                    log::debug!("Accepted connection from {}", peer_addr);
                    let router = Router::new(config.clone());
                    thread::spawn(move || {
                        Self::handle_connection(stream, router);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(ACCEPT_POLL_INTERVAL);
                }
                Err(e) => {
                    log::error!("Accept error: {}", e);
                    break;
                }
            }
        }
        log::debug!("Server accept loop terminated");
    }

    fn handle_connection(mut stream: TcpStream, router: Router) {
        if let Err(e) = Self::process_requests(&mut stream, router) {
            log::error!("Error handling connection: {}", e);
        }
    }

    fn process_requests(stream: &mut TcpStream, router: Router) -> Result<()> {
        stream.set_read_timeout(Some(PERSISTENT_CONNECTION_READ_TIMEOUT))?;
        loop {
            if let Some(request) = parse_request(stream).context("Failed to parse request")? {
                let response = router.handle(&request);
                stream
                    .write_all(&response.to_bytes())
                    .context("Failed to write response")?;
                if let Some(connection_header) = request.headers.get("connection") {
                    if connection_header == "close" {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}
