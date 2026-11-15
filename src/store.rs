// Disposable, authenticated memo storage. This host adapter never grants a verdict.
// The pure reuse module authenticates entries before consumers see their payload.
use crate::{reuse, sha256};

pub struct Store {
    directory: Directory,
    clock: Clock,
    producer: Vec<u8>,
    secret: Vec<u8>,
    sequence: usize,
    pub hits: usize,
    pub misses: usize,
    pub rejected: usize,
    pub writes: usize,
}

fn location(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(0) => ("/", &path[1..]),
        Some(at) => (&path[..at], &path[at + 1..]),
        None => (".", path),
    }
}

impl Store {
    pub fn open(world: &mut World, path: &Option<String>, key: &Option<String>) -> Option<Self> {
        let path = path.as_ref()?;
        let key = key.as_ref()?;
        let (parent, name) = location(key);
        let mut owner = world.directory(parent).ok()?;
        let mut key_file = owner.open(name).ok()?;
        let mut secret = vec![0u8; 33];
        let mut length = 0usize;
        while length < secret.len() {
            let count = key_file.read(&mut secret[length..]).ok()?;
            if count == 0 { break; }
            length += count;
        }
        if length != 32 { return None; }
        secret.pop();
        let directory = world.directory(path).ok()?;
        let mut executable = world.executable().ok()?;
        let mut hash = sha256::Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let count = executable.read(&mut buffer).ok()?;
            if count == 0 { break; }
            hash.update(&buffer[..count]);
        }
        let producer = hash.finish();
        Some(Self {
            directory, clock: world.clock(), producer: producer[..].to_vec(), secret,
            sequence: 0, hits: 0, misses: 0, rejected: 0, writes: 0,
        })
    }

    fn raw(&mut self, namespace: &str, key: &str) -> Option<Vec<u8>> {
        let name = format!("{namespace}-{key}");
        let mut file = self.directory.open(&name).ok()?;
        let result = read_envelope(&mut file, &self.producer, &self.secret, namespace, key);
        if result.is_none() { self.rejected += 1; }
        result
    }

    pub fn get(&mut self, namespace: &str, key: &str) -> Option<Vec<u8>> {
        let raw = match self.raw(namespace, key) {
            Some(raw) => raw,
            None => { self.misses += 1; return None; }
        };
        match reuse::authenticate(&self.producer, &self.secret, namespace, key, &raw) {
            Some(payload) => { self.hits += 1; Some(payload) }
            None => { self.rejected += 1; self.misses += 1; None }
        }
    }

    pub fn records(&mut self, namespace: &str, keys: &[String]) -> reuse::Records {
        let mut records = reuse::Records::new(&self.producer, &self.secret);
        for key in keys {
            if let Some(raw) = self.raw(namespace, key) {
                records.load(namespace, key, &raw);
            }
        }
        records
    }

    fn publish(&mut self, namespace: &str, key: &str, bytes: &[u8]) {
        let stamp = match self.clock.nanoseconds() { Ok(stamp) => stamp, Err(_) => return };
        let temporary: String = loop {
            self.sequence = self.sequence.wrapping_add(1);
            let name = format!(".tmp-{stamp}-{}", self.sequence);
            match self.directory.create_directory(&name) {
                Ok(()) => break name,
                Err(17) => continue,
                Err(_) => return,
            }
        };
        let mut written = false;
        if let Ok(mut directory) = self.directory.open_directory(&temporary) {
            if let Ok(mut file) = directory.create("entry") {
                written = file.write_all(bytes).is_ok();
            }
        }
        let from = format!("{temporary}/entry");
        if written {
            if self.directory.rename(&from, &format!("{namespace}-{key}")).is_ok() {
                self.writes += 1;
            }
        }
        let _ = self.directory.remove(&from);
        let _ = self.directory.remove_directory(&temporary);
    }

    pub fn put(&mut self, namespace: &str, key: &str, payload: &[u8]) {
        let envelope = reuse::seal(&self.producer, &self.secret, namespace, key, payload);
        self.publish(namespace, key, &envelope);
    }

    pub fn flush(&mut self, records: &reuse::Records) {
        self.hits += records.hits;
        self.misses += records.misses;
        self.rejected += records.rejected;
        for pending in records.pending() {
            self.publish(&pending.namespace, &pending.key, &pending.bytes);
        }
    }
}

// Every loader request is recorded, including absent alternative module paths.
// Embedded sources are bound by the producer identity instead of reread here.
#[derive(Clone)]
pub struct Input {
    pub directory: String,
    pub path: String,
    pub digest: Option<String>,
}

pub fn input(directory: &str, path: &str, bytes: &Option<Vec<u8>>) -> Input {
    Input {
        directory: directory.to_string(), path: path.to_string(),
        digest: bytes.as_ref().map(|bytes: &Vec<u8>| sha256::hex_digest(bytes)),
    }
}

pub fn push_word(out: &mut Vec<u8>, value: usize) {
    for shift in 0..8usize { out.push((value >> (shift * 8)) as u8); }
}
pub fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    push_word(out, bytes.len());
    out.extend_from_slice(bytes);
}
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self { Self { bytes, at: 0 } }
    pub fn word(&mut self) -> Option<usize> {
        if self.bytes.len() - self.at < 8 { return None; }
        let mut result = 0usize;
        for shift in 0..8usize { result |= (self.bytes[self.at + shift] as usize) << (shift * 8); }
        self.at += 8;
        Some(result)
    }
    pub fn bytes(&mut self) -> Option<Vec<u8>> {
        let length = self.word()?;
        if length > self.bytes.len() - self.at { return None; }
        let result = self.bytes[self.at..self.at + length].to_vec();
        self.at += length;
        Some(result)
    }
    pub fn text(&mut self) -> Option<String> { String::from_utf8(self.bytes()?).ok() }
    pub fn done(&self) -> bool { self.at == self.bytes.len() }
}

pub fn encode_inputs(inputs: &[Input]) -> Vec<u8> {
    let mut out = Vec::new();
    push_word(&mut out, inputs.len());
    for input in inputs {
        push_bytes(&mut out, input.directory.as_bytes());
        push_bytes(&mut out, input.path.as_bytes());
        match &input.digest {
            Some(digest) => push_bytes(&mut out, digest.as_bytes()),
            None => push_bytes(&mut out, b""),
        }
    }
    out
}

pub fn inputs_match(world: &mut World, bytes: &[u8]) -> bool {
    let mut reader = Reader::new(bytes);
    let count = match reader.word() { Some(count) => count, None => return false };
    // Each record has three length words even when every string is empty.
    if count > bytes.len() / 24 { return false; }
    for _ in 0..count {
        let root = match reader.text() { Some(root) => root, None => return false };
        let path = match reader.text() { Some(path) => path, None => return false };
        let digest = match reader.text() { Some(digest) => digest, None => return false };
        let mut directory = match world.directory(&root) { Ok(dir) => dir, Err(_) => return false };
        match directory.open(&path) {
            Ok(mut file) => {
                if digest.is_empty() { return false; }
                let current = match file.read_to_end() { Ok(bytes) => bytes, Err(_) => return false };
                if sha256::hex_digest(&current) != digest { return false; }
            }
            Err(errno) => { if (errno != 2 && errno != 20) || !digest.is_empty() { return false; } }
        }
    }
    reader.done()
}

fn read_exact(file: &mut File, bytes: &mut [u8]) -> bool {
    let mut at = 0usize;
    while at < bytes.len() {
        let count = match file.read(&mut bytes[at..]) { Ok(count) => count, Err(_) => return false };
        if count == 0 { return false; }
        at += count;
    }
    true
}
fn at_end(file: &mut File) -> bool {
    let mut byte = [0u8; 1];
    match file.read(&mut byte) { Ok(0) => true, _ => false }
}
fn read_envelope(
    file: &mut File, producer: &[u8], secret: &[u8], namespace: &str, key: &str,
) -> Option<Vec<u8>> {
    let mut header = vec![0u8; reuse::header_size()];
    if !read_exact(file, &mut header) { return None; }
    let payload = reuse::payload_length(&header)?;
    let length = header.len().checked_add(payload)?.checked_add(32)?;
    // A small bounded buffer is an I/O optimization, never an admission limit.
    // Large attacker-controlled lengths are streamed before any sized allocation.
    if length > 8192 {
        let mut verifier = reuse::Verifier::new(producer, secret, namespace, key, payload)?;
        let mut buffer = [0u8; 8192];
        let mut remaining = payload;
        while remaining != 0 {
            let count = remaining.min(buffer.len());
            if !read_exact(file, &mut buffer[..count]) { return None; }
            if !verifier.update(&buffer[..count]) { return None; }
            remaining -= count;
        }
        let mut tag = [0u8; 32];
        if !read_exact(file, &mut tag) || !at_end(file) || !verifier.finish(&tag) {
            return None;
        }
        file.rewind().ok()?;
        let mut bytes = vec![0u8; length];
        if !read_exact(file, &mut bytes) || !at_end(file) { return None; }
        // get()/Records::load authenticate this second snapshot again: changing
        // the file between passes cannot substitute a different payload.
        Some(bytes)
    } else {
        let mut tail = vec![0u8; length - header.len()];
        if !read_exact(file, &mut tail) || !at_end(file) { return None; }
        header.extend_from_slice(&tail);
        Some(header)
    }
}
