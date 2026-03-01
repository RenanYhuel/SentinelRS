use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use super::record::Record;

pub struct Segment {
    path: PathBuf,
    writer: BufWriter<File>,
    count: usize,
}

impl Segment {
    pub fn create(dir: &Path, index: u64) -> io::Result<Self> {
        let path = dir.join(format!("wal-{:07}.log", index));
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            path,
            writer: BufWriter::new(file),
            count: 0,
        })
    }

    pub fn append(&mut self, record: &Record, fsync: bool) -> io::Result<()> {
        let bytes = record.encode();
        self.writer.write_all(&bytes)?;
        self.writer.flush()?;
        if fsync {
            self.writer.get_ref().sync_data()?;
        }
        self.count += 1;
        Ok(())
    }

    pub fn read_all(path: &Path) -> io::Result<Vec<Record>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut records = Vec::new();
        loop {
            match Record::decode(&mut reader) {
                Ok(r) => records.push(r),
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
        }
        Ok(records)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn size_bytes(&self) -> io::Result<u64> {
        Ok(fs::metadata(&self.path)?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_segment() {
        let dir = tempfile::tempdir().unwrap();
        let mut seg = Segment::create(dir.path(), 1).unwrap();

        seg.append(
            &Record { id: 0, data: b"first".to_vec() },
            true,
        ).unwrap();
        seg.append(
            &Record { id: 1, data: b"second".to_vec() },
            true,
        ).unwrap();

        let records = Segment::read_all(seg.path()).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].data, b"first");
        assert_eq!(records[1].data, b"second");
    }
}
