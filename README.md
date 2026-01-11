[![progress-banner](https://backend.codecrafters.io/progress/http-server/6b30c19c-f09d-4257-bb77-0b542ee31f98)](https://app.codecrafters.io/users/codecrafters-bot?r=2qF)

This is a well-structured, modular implementation of a
["Build Your Own HTTP server" Challenge](https://app.codecrafters.io/courses/http-server/overview).

[HTTP](https://en.wikipedia.org/wiki/Hypertext_Transfer_Protocol) is the
protocol that powers the web. This HTTP/1.1 server is capable of serving multiple clients
concurrently.

Along the way you'll learn about TCP servers,
[HTTP request syntax](https://www.w3.org/Protocols/rfc2616/rfc2616-sec5.html),
and more.

**Note**: If you're viewing this repo on GitHub, head over to
[codecrafters.io](https://codecrafters.io) to try the challenge.

## Project Structure

This project follows Rust best practices with a modular architecture:

```
src/
├── main.rs              # Entry point - orchestrates modules
├── http/                # HTTP protocol implementation
│   ├── mod.rs          # Module exports
│   ├── types.rs        # HTTP types (Method, Request, Response, Headers)
│   └── parser.rs       # Request parsing logic
├── compression.rs       # Gzip compression utilities
├── config.rs           # Configuration and argument parsing
└── handlers/           # Request handlers organized by functionality
    ├── mod.rs          # Router and handler dispatch
    ├── echo.rs         # /echo/* endpoint
    ├── user_agent.rs   # /user-agent endpoint
    ├── file.rs         # /files/* endpoint (GET/POST)
    └── root.rs         # / endpoint
```

For detailed information about the refactoring, see [REFACTORING_SUMMARY.md](REFACTORING_SUMMARY.md).

## Features

- ✅ HTTP/1.1 support with multiple HTTP methods (GET, POST, PUT, DELETE)
- ✅ Multi-threaded connection handling using Rust threads
- ✅ Gzip compression support for `/echo/*` endpoint
- ✅ File upload and download with `/files/*` endpoints
- ✅ User-Agent header extraction
- ✅ 27 comprehensive unit tests with 100% pass rate
- ✅ Idiomatic Rust code following best practices

## Running the Server

### Prerequisites

- Rust 1.70 or later
- Cargo

### Basic Usage

```sh
cargo run
```

The server will start on `127.0.0.1:4221`

### Logging

This project uses the Rust standard `log` crate with `env_logger` for logging. Control logging level via the `RUST_LOG` environment variable:

```sh
# Show info and error messages
RUST_LOG=info cargo run

# Show debug information (very verbose)
RUST_LOG=debug cargo run

# Show only errors
RUST_LOG=error cargo run

# Show logs from specific modules
RUST_LOG=codecrafters_http_server::handlers=debug cargo run
```

### With File Directory

```sh
cargo run -- --directory /tmp/files
# or
cargo run -- -d /tmp/files
```

### Building Release Binary

```sh
cargo build --release
./target/release/codecrafters-http-server --directory /tmp/files
```

## Testing

### Run All Tests

```sh
cargo test
```

### Run Specific Module Tests

```sh
cargo test http::types
cargo test handlers::echo
cargo test handlers::file
```

## API Endpoints

### GET /

Returns 200 OK with empty body.

```bash
curl http://127.0.0.1:4221/
```

### GET /echo/<text>

Echoes the provided text. Supports gzip compression if `Accept-Encoding: gzip` header is present.

```bash
curl http://127.0.0.1:4221/echo/hello

curl -H "Accept-Encoding: gzip" http://127.0.0.1:4221/echo/hello
```

### GET /user-agent

Returns the value of the `User-Agent` header from the request.

```bash
curl -H "User-Agent: MyAgent" http://127.0.0.1:4221/user-agent
```

### GET /files/<filename>

Retrieves a file from the configured directory (requires `--directory` flag).

```bash
curl http://127.0.0.1:4221/files/myfile.txt
```

Returns 200 with file content, or 404 if file not found.

### POST /files/<filename>

Uploads/creates a file in the configured directory (requires `--directory` flag).

```bash
curl --data-binary @local_file.txt http://127.0.0.1:4221/files/remote_file.txt
```

Returns 201 Created on success, or 404 if directory not configured.

# Passing the first stage

The entry point for your HTTP server implementation is in `src/main.rs`. Study
and uncomment the relevant code, and push your changes to pass the first stage:

```sh
git commit -am "pass 1st stage" # any msg
git push origin master
```

Time to move on to the next stage!

# Stage 2 & beyond

Note: This section is for stages 2 and beyond.

1. Ensure you have `cargo` installed locally
1. Run `./your_program.sh` to run your program, which is implemented in
   `src/main.rs`. This command compiles your Rust project, so it might be slow
   the first time you run it. Subsequent runs will be fast.
1. Commit your changes and run `git push origin master` to submit your solution
   to CodeCrafters. Test output will be streamed to your terminal.
