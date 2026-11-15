//! Append-only journal and atomic durable-head manifest.
use std::sha256::Sha256;

const LOG_MAGIC: &[u8] = b"LKJLOG01";
const HEAD_MAGIC: &[u8] = b"LKJHEAD1";

#[derive(Clone)]
pub struct Entry {
    pub id: usize,
    pub revision: usize,
    pub text: String,
}
struct Record {
    entry: Entry,
    operation: u64,
    bytes: usize,
    digest: [u8; 32],
}
enum Frame { End, Incomplete, Complete(Record) }
struct Head {
    end: usize,
    revision: usize,
    next_id: usize,
    digest: [u8; 32],
}
pub struct Store {
    root: Directory,
    log: File,
    lock: File,
    entries: Vec<Entry>,
    end: usize,
    revision: usize,
    digest: [u8; 32],
}

fn io<T>(result: Result<T, i64>, action: &str) -> Result<T, String> {
    match result {
        Ok(value) => Ok(value),
        Err(errno) => Err(format!("{action}: errno {errno}")),
    }
}
fn word(bytes: &[u8], at: usize) -> u64 {
    let mut value = 0u64;
    for index in 0usize..8usize {
        value |= (bytes[at + index] as u64) << ((index * 8usize) as u64);
    }
    value
}
fn put_word(bytes: &mut [u8], at: usize, value: u64) {
    for index in 0usize..8usize {
        bytes[at + index] = (value >> ((index * 8usize) as u64)) as u8;
    }
}
fn header(length: usize, id: usize, revision: usize, operation: u64) -> [u8; 40] {
    let mut bytes = [0u8; 40];
    bytes[..8].copy_from_slice(LOG_MAGIC);
    put_word(&mut bytes, 8usize, length as u64);
    put_word(&mut bytes, 16usize, id as u64);
    put_word(&mut bytes, 24usize, revision as u64);
    put_word(&mut bytes, 32usize, operation);
    bytes
}
fn read_part(file: &mut File, bytes: &mut [u8]) -> Result<usize, String> {
    let mut at = 0usize;
    while at < bytes.len() {
        let count = io(file.read(&mut bytes[at..]), "read journal")?;
        if count == 0usize { break; }
        at += count;
    }
    Ok(at)
}
fn read_frame(log: &mut File) -> Result<Frame, String> {
    let mut bytes = [0u8; 40];
    let count = read_part(log, &mut bytes)?;
    if count == 0usize { return Ok(Frame::End); }
    if count != bytes.len() { return Ok(Frame::Incomplete); }
    if &bytes[..8] != LOG_MAGIC { return Err("journal record has invalid magic".to_string()); }
    let length = word(&bytes, 8usize) as usize;
    // SHA-256's bit-length representation, not an application record-size policy.
    if length > 2_305_843_009_213_693_911usize {
        return Err("journal record exceeds SHA-256 length representation".to_string());
    }
    let size = length.checked_add(72usize).ok_or("journal record length overflow".to_string())?;
    let id = word(&bytes, 16usize) as usize;
    let revision = word(&bytes, 24usize) as usize;
    let operation = word(&bytes, 32usize);
    let mut hash = Sha256::new();
    hash.update(&bytes);
    let mut text: Vec<u8> = Vec::new();
    let mut left = length;
    while left != 0usize {
        let mut chunk = [0u8; 8192];
        let wanted = if left < chunk.len() { left } else { chunk.len() };
        let count = io(log.read(&mut chunk[..wanted]), "read journal text")?;
        if count == 0usize { return Ok(Frame::Incomplete); }
        hash.update(&chunk[..count]);
        text.extend_from_slice(&chunk[..count]);
        left -= count;
    }
    let mut digest = [0u8; 32];
    if read_part(log, &mut digest)? != digest.len() { return Ok(Frame::Incomplete); }
    if hash.finish() != digest { return Err("journal record checksum mismatch".to_string()); }
    let text = match String::from_utf8(text) {
        Ok(text) => text,
        Err(_) => return Err("journal text is not UTF-8".to_string()),
    };
    Ok(Frame::Complete(Record { entry: Entry { id, revision, text }, operation, bytes: size, digest }))
}
fn head_bytes(end: usize, revision: usize, next_id: usize, digest: &[u8; 32]) -> [u8; 96] {
    let mut bytes = [0u8; 96];
    bytes[..8].copy_from_slice(HEAD_MAGIC);
    put_word(&mut bytes, 8usize, end as u64);
    put_word(&mut bytes, 16usize, revision as u64);
    put_word(&mut bytes, 24usize, next_id as u64);
    bytes[32..64].copy_from_slice(digest);
    let mut hash = Sha256::new();
    hash.update(&bytes[..64]);
    bytes[64..].copy_from_slice(&hash.finish());
    bytes
}
fn parse_head(bytes: &[u8]) -> Result<Head, String> {
    if bytes.len() != 96usize || &bytes[..8] != HEAD_MAGIC {
        return Err("journal head has invalid framing".to_string());
    }
    let mut hash = Sha256::new();
    hash.update(&bytes[..64]);
    if &hash.finish()[..] != &bytes[64..] {
        return Err("journal head checksum mismatch".to_string());
    }
    let mut digest = [0u8; 32];
    digest[..].copy_from_slice(&bytes[32..64]);
    Ok(Head { end: word(bytes, 8usize) as usize, revision: word(bytes, 16usize) as usize,
        next_id: word(bytes, 24usize) as usize, digest })
}
fn read_head(root: &mut Directory) -> Result<Option<Head>, String> {
    let mut file = match root.open("head") {
        Ok(file) => file,
        Err(2i64) => return Ok(None),
        Err(errno) => return Err(format!("open journal head: errno {errno}")),
    };
    let mut bytes = [0u8; 97];
    let count = read_part(&mut file, &mut bytes)?;
    Ok(Some(parse_head(&bytes[..count])?))
}
impl Store {
    pub fn open(mut root: Directory, output: &mut Stdout) -> Result<Store, String> {
        let mut lock = io(root.open_append("lock"), "open journal lock")?;
        io(lock.lock_exclusive(), "lock journal")?;
        let log = io(root.open_append("journal.log"), "open journal log")?;
        io(root.sync(), "sync journal directory")?;
        let head = read_head(&mut root)?;
        let mut store = Store { root, log, lock, entries: Vec::new(), end: 0usize,
            revision: 0usize, digest: [0u8; 32] };
        output.write(b"recovery begin\n");
        let mut matched = match &head {
            Some(head) => head.end == 0usize && head.revision == 0usize
                && head.next_id == 1usize && head.digest == [0u8; 32],
            None => true,
        };
        loop {
            let frame = read_frame(&mut store.log)?;
            match frame {
                Frame::End => break,
                Frame::Incomplete => {
                    if !matched {
                        return Err("journal head points beyond the complete log prefix; no repair performed".to_string());
                    }
                    io(store.log.truncate(store.end), "truncate incomplete journal tail")?;
                    output.write(format!("recovery repaired {}\n", store.end).as_bytes());
                    break;
                }
                Frame::Complete(record) => {
                    let expected = store.revision.checked_add(1usize).ok_or("journal revision overflow".to_string())?;
                    if record.entry.revision != expected {
                        return Err("journal revision sequence is not contiguous".to_string());
                    }
                    let id = record.entry.id;
                    let next_id = store.entries.len().checked_add(1usize).ok_or("journal entry count overflow".to_string())?;
                    if record.operation == 1u64 && id == next_id {
                        store.entries.push(record.entry);
                    } else if record.operation == 2u64 && id != 0usize && id < next_id {
                        store.entries[id - 1usize] = record.entry;
                    } else {
                        return Err("journal record has invalid operation or entry id".to_string());
                    }
                    store.end = store.end.checked_add(record.bytes).ok_or("journal file offset overflow".to_string())?;
                    store.revision = expected;
                    store.digest = record.digest;
                    if let Some(head) = &head {
                        if head.end == store.end {
                            if head.revision != store.revision || head.next_id != store.next_id()
                                || head.digest != store.digest {
                                return Err("journal head does not match its log boundary".to_string());
                            }
                            matched = true;
                        }
                    }
                    if store.revision == 1usize || store.revision % 128usize == 0usize {
                        output.write(format!("recovery record {} {}\n", store.revision, store.end).as_bytes());
                    }
                }
            }
        }
        if !matched { return Err("journal head points beyond a valid log boundary".to_string()); }
        // Adopted complete records and any repair become durable before serving.
        io(store.log.sync(), "sync recovered journal")?;
        match store.root.remove(".head-next") {
            Ok(()) | Err(2i64) => {},
            Err(errno) => return Err(format!("remove abandoned journal head: errno {errno}")),
        }
        store.publish_head()?;
        output.write(format!("recovery done {} {}\n", store.revision, store.end).as_bytes());
        Ok(store)
    }
    fn publish_head(&mut self) -> Result<(), String> {
        let bytes = head_bytes(self.end, self.revision, self.next_id(), &self.digest);
        let mut file = io(self.root.create_new(".head-next"), "create journal head")?;
        io(file.write_all(&bytes), "write journal head")?;
        io(file.sync(), "sync journal head")?;
        drop(file);
        io(self.root.rename(".head-next", "head"), "replace journal head")?;
        io(self.root.sync(), "sync journal head directory")?;
        Ok(())
    }
    pub fn revision(&self) -> usize { self.revision }
    pub fn next_id(&self) -> usize { self.entries.len() + 1usize }
    pub fn entries(&self) -> &[Entry] { &self.entries }
    pub fn get(&self, id: usize) -> Option<&Entry> {
        if id == 0usize || id > self.entries.len() { None } else { Some(&self.entries[id - 1usize]) }
    }
    fn commit(&mut self, id: usize, text: String, append: bool) -> Result<(), String> {
        let revision = self.revision.checked_add(1usize).ok_or("journal revision overflow".to_string())?;
        let next_id = self.entries.len().checked_add(2usize).ok_or("journal entry count overflow".to_string())?;
        if append && id + 1usize != next_id { return Err("invalid journal append id".to_string()); }
        if !append && (id == 0usize || id >= self.next_id()) { return Err("missing journal entry".to_string()); }
        if text.len() > 2_305_843_009_213_693_911usize {
            return Err("journal record exceeds SHA-256 length representation".to_string());
        }
        let size = text.len().checked_add(72usize).ok_or("journal record length overflow".to_string())?;
        let end = self.end.checked_add(size).ok_or("journal file offset overflow".to_string())?;
        let bytes = header(text.len(), id, revision, if append { 1u64 } else { 2u64 });
        let mut hash = Sha256::new();
        hash.update(&bytes);
        hash.update(text.as_bytes());
        let digest = hash.finish();
        io(self.log.write_all(&bytes), "append journal header")?;
        io(self.log.write_all(text.as_bytes()), "append journal text")?;
        io(self.log.write_all(&digest), "append journal checksum")?;
        io(self.log.sync(), "sync journal commit")?;
        let entry = Entry { id, revision, text };
        if append { self.entries.push(entry); } else { self.entries[id - 1usize] = entry; }
        self.end = end;
        self.revision = revision;
        self.digest = digest;
        self.publish_head()
    }
    pub fn append(&mut self, text: String) -> Result<usize, String> {
        let id = self.next_id();
        self.commit(id, text, true)?;
        Ok(id)
    }
    pub fn replace(&mut self, id: usize, text: String) -> Result<(), String> {
        self.commit(id, text, false)
    }
}

#[test]
fn binary_frames_and_head_integrity() {
    let bytes = header(17usize, 3usize, 9usize, 2u64);
    assert!(&bytes[..8] == LOG_MAGIC);
    assert!(word(&bytes, 8usize) == 17u64 && word(&bytes, 16usize) == 3u64);
    assert!(word(&bytes, 24usize) == 9u64 && word(&bytes, 32usize) == 2u64);
    let digest = std::sha256::digest(b"journal");
    let encoded = head_bytes(401usize, 9usize, 4usize, &digest);
    let decoded = parse_head(&encoded).unwrap();
    assert!(decoded.end == 401usize && decoded.revision == 9usize && decoded.next_id == 4usize);
    assert!(decoded.digest == digest);
    assert!(parse_head(&encoded[..95]).is_err());
    let mut damaged = encoded;
    damaged[16] ^= 1u8;
    assert!(parse_head(&damaged).is_err());
}
