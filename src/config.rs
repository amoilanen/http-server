use std::env;

/// Server configuration parsed from command-line arguments
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Optional directory for file operations
    pub directory: Option<String>,
}

impl ServerConfig {
    #[allow(dead_code)]
    pub fn new(directory: Option<String>) -> Self {
        ServerConfig { directory }
    }
}

/// Parse command-line arguments into ServerConfig
pub fn parse_args() -> Result<ServerConfig, String> {
    let mut directory: Option<String> = None;
    let args: Vec<String> = env::args().collect();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--directory" => {
                if i + 1 < args.len() {
                    directory = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err("--directory flag requires a value".to_string());
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(ServerConfig { directory })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_new() {
        let config = ServerConfig::new(Some("/tmp".to_string()));
        assert_eq!(config.directory, Some("/tmp".to_string()));
    }

    #[test]
    fn test_server_config_no_directory() {
        let config = ServerConfig::new(None);
        assert_eq!(config.directory, None);
    }
}

