//! Owned byte snapshots read directly into fixed-size allocation chunks.
//! Chunk capacity is a storage choice, not a file-size or request limit.
use crate::{CheckedArithmetic, Result};

struct Chunk {
    bytes: [u8; 8192],
    used: usize,
}
pub struct ReadBuffer {
    chunks: Vec<Chunk>,
    length: usize,
}
impl ReadBuffer {
    pub fn read(file: &mut File) -> Result<ReadBuffer, i64> {
        let mut chunks: Vec<Chunk> = Vec::new();
        let mut length = 0usize;
        loop {
            let mut bytes = [0u8; 8192];
            let used = file.read(&mut bytes)?;
            if used == 0usize { break; }
            length = length.checked_add(used).ok_or(75i64)?;
            chunks.push(Chunk { bytes, used });
        }
        Result::Ok(ReadBuffer { chunks, length })
    }
    pub fn len(&self) -> usize { self.length }
    pub fn is_empty(&self) -> bool { self.length == 0usize }
    pub fn write_to(&self, file: &mut File) -> Result<(), i64> {
        for chunk in &self.chunks {
            file.write_all(&chunk.bytes[..chunk.used])?;
        }
        Result::Ok(())
    }
}
