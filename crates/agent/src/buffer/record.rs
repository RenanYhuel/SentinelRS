use std::io::{self, Read};

pub struct Record {
    pub id: u64,
    pub data: Vec<u8>,
}

impl Record {
    pub fn encode(&self) -> Vec<u8> {
        let len = self.data.len() as u32;
        let crc = crc32fast::hash(&self.data);
        let mut buf = Vec::with_capacity(4 + 8 + self.data.len() + 4);
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(&self.id.to_le_bytes());
        buf.extend_from_slice(&self.data);
        buf.extend_from_slice(&crc.to_le_bytes());
        buf
    }

    pub fn decode(reader: &mut impl Read) -> io::Result<Self> {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut id_buf = [0u8; 8];
        reader.read_exact(&mut id_buf)?;
        let id = u64::from_le_bytes(id_buf);

        let mut data = vec![0u8; len];
        reader.read_exact(&mut data)?;

        let mut crc_buf = [0u8; 4];
        reader.read_exact(&mut crc_buf)?;
        let stored_crc = u32::from_le_bytes(crc_buf);
        let actual_crc = crc32fast::hash(&data);

        if stored_crc != actual_crc {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "CRC mismatch"));
        }

        Ok(Record { id, data })
    }
}

impl Record {
    pub fn encoded_size(&self) -> usize {
        4 + 8 + self.data.len() + 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn roundtrip() {
        let rec = Record {
            id: 42,
            data: b"hello world".to_vec(),
        };
        let encoded = rec.encode();
        let mut cursor = Cursor::new(encoded);
        let decoded = Record::decode(&mut cursor).unwrap();
        assert_eq!(decoded.id, 42);
        assert_eq!(decoded.data, b"hello world");
    }

    #[test]
    fn crc_mismatch_detected() {
        let rec = Record {
            id: 1,
            data: b"data".to_vec(),
        };
        let mut encoded = rec.encode();
        let last = encoded.len() - 1;
        encoded[last] ^= 0xFF;
        let mut cursor = Cursor::new(encoded);
        assert!(Record::decode(&mut cursor).is_err());
    }
}
