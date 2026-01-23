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
    fn test_gzip_encode() {
        let data = b"Hello, World!";
        let compressed = gzip_encode(data).unwrap();

        // Gzip compressed data should be different from original
        assert_ne!(&compressed[..], data);
        assert_eq!(compressed[0..2], [0x1f, 0x8b]);
    }
}

