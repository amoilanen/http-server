use anyhow::Result;

mod args;
mod http;
mod compression;
mod config;
mod handlers;
mod server;

use config::parse_args;
use server::Server;

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .init();

    let server_config = parse_args()?;
    log::info!("Server configuration: {:?}", server_config);

    let mut server = Server::start("127.0.0.1:4221", server_config)?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
    });

    log::info!("Received shutdown signal, stopping server...");
    server.shutdown();

    Ok(())
}
