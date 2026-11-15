//! Owned byte snapshots read directly into fixed-size allocation chunks.
//! Chunk capacity is a storage choice, not a file-size or request limit.
use crate::{CheckedArithmetic, Result};

struct Chunk {
    bytes: [u8; 8192],
    used: usize,
}
pub struct WriteCursor {
    chunk: usize,
    offset: usize,
    position: usize,
}
impl WriteCursor {
    pub fn new() -> WriteCursor { WriteCursor { chunk: 0usize, offset: 0usize, position: 0usize } }
    pub fn position(&self) -> usize { self.position }
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
    /// Make at most one write call; preserve the cursor on would-block.
    pub fn write_some(&self, file: &mut File, cursor: &mut WriteCursor) -> Result<usize, i64> {
        if cursor.chunk == self.chunks.len() { return Result::Ok(0usize); }
        if cursor.chunk > self.chunks.len() { return Result::Err(22i64); }
        let chunk = &self.chunks[cursor.chunk];
        if cursor.offset >= chunk.used { return Result::Err(22i64); }
        let count = file.write(&chunk.bytes[cursor.offset..chunk.used])?;
        if count == 0usize { return Result::Err(5i64); }
        cursor.offset += count;
        cursor.position += count;
        if cursor.offset == chunk.used {
            cursor.chunk += 1usize;
            cursor.offset = 0usize;
        }
        Result::Ok(count)
    }
    pub fn write_to(&self, file: &mut File) -> Result<(), i64> {
        for chunk in &self.chunks {
            file.write_all(&chunk.bytes[..chunk.used])?;
        }
        Result::Ok(())
    }
}
