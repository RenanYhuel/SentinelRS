use std::io::{Read, Write};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

const COMPRESS_THRESHOLD: usize = 1024;

pub fn should_compress(data: &[u8]) -> bool {
    data.len() >= COMPRESS_THRESHOLD
}

pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data).expect("gzip write");
    encoder.finish().expect("gzip finish")
}

pub fn decompress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_decompress_roundtrip() {
        let original = b"hello world repeated ".repeat(100);
        let compressed = compress(&original);
        let restored = decompress(&compressed).unwrap();
        assert_eq!(original.as_slice(), restored.as_slice());
    }

    #[test]
    fn compression_reduces_size() {
        let data = b"aaaaaaaaaa".repeat(200);
        let compressed = compress(&data);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn threshold_check() {
        assert!(!should_compress(&[0u8; 512]));
        assert!(should_compress(&[0u8; 1024]));
    }
}
