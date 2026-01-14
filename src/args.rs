use std::collections::HashMap;
use anyhow::{bail, Result};

fn process_option(
    prefix: &str,
    args: &[String],
    i: usize,
    options: &mut HashMap<String, String>,
) -> Result<usize> {
    let arg = &args[i];
    let key = arg.trim_start_matches(prefix);
    
    if key.is_empty() {
        bail!("Invalid option: {}", arg);
    }

    if i + 1 < args.len() && !args[i + 1].starts_with('-') {
        // Next argument is the value
        options.insert(key.to_string(), args[i + 1].clone());
        Ok(i + 2)
    } else {
        bail!("Option {} requires a value", arg);
    }
}

pub fn parse_args(args: &[String]) -> Result<HashMap<String, String>> {
    let mut options = HashMap::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        // Special case: "--" signals end of options, stop parsing
        if arg == "--" {
            break;
        }

        // Special case: "-" is treated as a positional argument (stdin/stdout convention)
        if arg == "-" {
            i += 1;
            continue;
        }

        if arg.starts_with("--") {
            // Long option (e.g., --directory)
            i = process_option("--", args, i, &mut options)?;
        } else if arg.starts_with('-') && arg.len() > 1 {
            // Short option (e.g., -d)
            i = process_option("-", args, i, &mut options)?;
        } else {
            // Not an option, skip positional argument
            i += 1;
        }
    }

    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_short_option() {
        let args = vec![
            "program".to_string(),
            "-d".to_string(),
            "/tmp".to_string(),
        ];
        let result = parse_args(&args).unwrap();
        assert_eq!(result.get("d"), Some(&"/tmp".to_string()));
    }

    #[test]
    fn test_parse_long_option() {
        let args = vec![
            "program".to_string(),
            "--directory".to_string(),
            "/tmp".to_string(),
        ];
        let result = parse_args(&args).unwrap();
        assert_eq!(result.get("directory"), Some(&"/tmp".to_string()));
    }

    #[test]
    fn test_parse_multiple_options() {
        let args = vec![
            "program".to_string(),
            "-d".to_string(),
            "/tmp".to_string(),
            "--port".to_string(),
            "8080".to_string(),
        ];
        let result = parse_args(&args).unwrap();
        assert_eq!(result.get("d"), Some(&"/tmp".to_string()));
        assert_eq!(result.get("port"), Some(&"8080".to_string()));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_empty_args() {
        let args = vec!["program".to_string()];
        let result = parse_args(&args).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_no_args() {
        let args: Vec<String> = vec![];
        let result = parse_args(&args).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_option_without_value_short() {
        let args = vec!["program".to_string(), "-d".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a value"));
    }

    #[test]
    fn test_option_without_value_long() {
        let args = vec!["program".to_string(), "--directory".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a value"));
    }

    #[test]
    fn test_option_followed_by_another_option() {
        let args = vec![
            "program".to_string(),
            "-d".to_string(),
            "-p".to_string(),
        ];
        let result = parse_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a value"));
    }

    #[test]
    fn test_single_dash_as_positional() {
        let args = vec!["program".to_string(), "-".to_string(), "value".to_string()];
        let result = parse_args(&args);
        // Single dash is treated as a positional argument (stdin/stdout convention)
        // and is skipped, not parsed as an option
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_double_dash_ends_option_parsing() {
        let args = vec![
            "program".to_string(),
            "-d".to_string(),
            "/tmp".to_string(),
            "--".to_string(),
            "-notanoption".to_string(),
        ];
        let result = parse_args(&args);
        // "--" signals end of options, so "-notanoption" is not parsed as an option
        assert!(result.is_ok());
        let options = result.unwrap();
        assert_eq!(options.get("d"), Some(&"/tmp".to_string()));
        assert_eq!(options.len(), 1); // Only one option parsed
    }

    #[test]
    fn test_double_dash_at_end() {
        let args = vec![
            "program".to_string(),
            "-d".to_string(),
            "/tmp".to_string(),
            "--".to_string(),
        ];
        let result = parse_args(&args);
        // "--" at the end should not cause an error
        assert!(result.is_ok());
        let options = result.unwrap();
        assert_eq!(options.get("d"), Some(&"/tmp".to_string()));
        assert_eq!(options.len(), 1);
    }

    #[test]
    fn test_duplicate_options_last_wins() {
        let args = vec![
            "program".to_string(),
            "-d".to_string(),
            "/tmp".to_string(),
            "-d".to_string(),
            "/var".to_string(),
        ];
        let result = parse_args(&args).unwrap();
        // Last value should win
        assert_eq!(result.get("d"), Some(&"/var".to_string()));
    }

    #[test]
    fn test_positional_arguments_are_skipped() {
        let args = vec![
            "program".to_string(),
            "positional1".to_string(),
            "-d".to_string(),
            "/tmp".to_string(),
            "positional2".to_string(),
        ];
        let result = parse_args(&args).unwrap();
        assert_eq!(result.get("d"), Some(&"/tmp".to_string()));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_option_with_equals_not_supported() {
        // Note: Our current implementation doesn't support --option=value format
        let args = vec!["program".to_string(), "--directory=/tmp".to_string()];
        let result = parse_args(&args);
        // This will fail because it expects a separate value
        assert!(result.is_err());
    }
}

