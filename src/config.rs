use std::env;
use anyhow::Result;
use crate::args;

/// Server configuration parsed from command-line arguments
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    /// Optional directory for file operations
    pub directory: Option<String>,
}

impl ServerConfig {
    pub fn new(directory: Option<String>) -> Self {
        ServerConfig { directory }
    }
}

/// Parse command-line arguments into ServerConfig
pub fn parse_args() -> Result<ServerConfig> {
    let args_vec: Vec<String> = env::args().collect();
    parse_args_from_vec(&args_vec)
}

/// Parse command-line arguments from a vector into ServerConfig
/// This function is separated for testability
fn parse_args_from_vec(args: &[String]) -> Result<ServerConfig> {
    let options = args::parse_args(args)?;
    
    // Check for directory option in both short (-d) and long (--directory) forms
    let directory = options.get("d")
        .or_else(|| options.get("directory"))
        .cloned();

    Ok(ServerConfig::new(directory))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_with_short_directory_flag() {
        let args = vec![
            "program".to_string(),
            "-d".to_string(),
            "/tmp".to_string(),
        ];
        let config = parse_args_from_vec(&args).unwrap();
        assert_eq!(config.directory, Some("/tmp".to_string()));
    }

    #[test]
    fn test_parse_args_with_long_directory_flag() {
        let args = vec![
            "program".to_string(),
            "--directory".to_string(),
            "/var/www".to_string(),
        ];
        let config = parse_args_from_vec(&args).unwrap();
        assert_eq!(config.directory, Some("/var/www".to_string()));
    }

    #[test]
    fn test_parse_args_no_directory() {
        let args = vec!["program".to_string()];
        let config = parse_args_from_vec(&args).unwrap();
        assert_eq!(config.directory, None);
    }

    #[test]
    fn test_parse_args_empty() {
        let args: Vec<String> = vec![];
        let config = parse_args_from_vec(&args).unwrap();
        assert_eq!(config.directory, None);
    }

    #[test]
    fn test_parse_args_with_other_args() {
        let args = vec![
            "program".to_string(),
            "some_positional".to_string(),
            "-d".to_string(),
            "/tmp".to_string(),
            "another_arg".to_string(),
        ];
        let config = parse_args_from_vec(&args).unwrap();
        assert_eq!(config.directory, Some("/tmp".to_string()));
    }

    #[test]
    fn test_parse_args_with_path_containing_spaces() {
        let args = vec![
            "program".to_string(),
            "-d".to_string(),
            "/path/to/my directory".to_string(),
        ];
        let config = parse_args_from_vec(&args).unwrap();
        assert_eq!(config.directory, Some("/path/to/my directory".to_string()));
    }

    #[test]
    fn test_parse_args_with_relative_path() {
        let args = vec![
            "program".to_string(),
            "-d".to_string(),
            "./relative/path".to_string(),
        ];
        let config = parse_args_from_vec(&args).unwrap();
        assert_eq!(config.directory, Some("./relative/path".to_string()));
    }
}

