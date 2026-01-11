use std::io::{Result as IoResult, Write};

use flate2::write::GzEncoder;
use flate2::Compression;

/// Compress data using gzip
pub fn gzip_encode(data: &[u8]) -> IoResult<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut encoder = GzEncoder::new(&mut buffer, Compression::default());
    encoder.write_all(data)?;
    encoder.finish()?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzip_encode_basic() {
        let data = b"Hello, World!";
        let compressed = gzip_encode(data).unwrap();
        
        // Gzip compressed data should be different from original
        assert_ne!(&compressed[..], data);
        
        // Should have gzip magic number (1f 8b)
        assert_eq!(compressed[0], 0x1f);
        assert_eq!(compressed[1], 0x8b);
    }

    #[test]
    fn test_gzip_encode_empty() {
        let data = b"";
        let compressed = gzip_encode(data).unwrap();
        
        // Even empty data should have gzip header/trailer
        assert!(compressed.len() > 0);
    }
}

