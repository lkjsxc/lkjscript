//! Independent acknowledgement oracle: replay every complete on-disk frame and
//! compare all HTTP-visible entries. Unacknowledged complete mutations may survive.
use crate::http_client::{io, require};
use std::json::Value;
use std::sha256::Sha256;

pub struct Attempt {
    pub id: usize,
    pub text: String,
    pub sent: bool,
    pub acknowledged_id: usize,
    pub acknowledged_revision: usize,
}
#[derive(Clone)]
pub struct Entry { pub id: usize, pub revision: usize, pub text: String }
pub struct Snapshot { pub entries: Vec<Entry>, pub revision: usize, pub bytes: usize }
pub struct Model {
    pub attempts: Vec<Attempt>,
    pub acknowledgements: usize,
    pub verified_frames: usize,
    pub snapshots: usize,
}
pub fn number(value: &Value, name: &str) -> Result<usize, String> {
    match value.get(name) {
        Some(value) => value.as_usize().ok_or(format!("missing unsigned {name}")),
        None => Err(format!("missing {name}")),
    }
}
pub fn text(value: &Value, name: &str) -> Result<String, String> {
    match value.get(name) {
        Some(value) => match value.as_str() {
            Some(text) => Ok(text.to_string()),
            None => Err(format!("invalid string {name}")),
        },
        None => Err(format!("missing {name}")),
    }
}
fn word(bytes: &[u8], at: usize) -> usize {
    let mut value = 0usize;
    for part in 0usize..8usize { value |= (bytes[at + part] as usize) << ((8usize * part) as u64); }
    value
}
fn read_part(file: &mut File, bytes: &mut [u8]) -> Result<usize, String> {
    let mut used = 0usize;
    while used < bytes.len() {
        let count = io(file.read(&mut bytes[used..]), "oracle read log")?;
        if count == 0usize { break; }
        used += count;
    }
    Ok(used)
}
fn serial(text: &str) -> Result<usize, String> {
    let mut parts = text.split(':');
    require(parts.next() == Some("op"), "log text was not issued by supervisor")?;
    let part = parts.next().ok_or("missing operation serial".to_string())?;
    part.parse::<usize>().map_err(|_| "invalid operation serial".to_string())
}
impl Model {
    pub fn new() -> Model {
        Model { attempts: Vec::new(), acknowledgements: 0usize, verified_frames: 0usize, snapshots: 0usize }
    }
    pub fn issue(&mut self, id: usize, size: usize) -> usize {
        let serial = self.attempts.len() + 1usize;
        let mut text = format!("op:{serial}:quote=\" slash=\\ newline=\n nul=\0 scalar=𝄞 中文 ");
        while text.len() < size {
            let letter = b'a' + ((serial + text.len()) % 26usize) as u8;
            text.push(letter as char);
        }
        self.attempts.push(Attempt { id, text, sent: false,
            acknowledged_id: 0usize, acknowledged_revision: 0usize });
        serial - 1usize
    }
    pub fn body(&self, index: usize) -> Result<Vec<u8>, String> {
        let value = Value::Object(vec![("text".to_string(), Value::String(self.attempts[index].text.clone()))]);
        std::json::stringify(&value).map(|text| text.into_bytes()).map_err(|problem| format!("{problem}"))
    }
    pub fn acknowledge(&mut self, index: usize, value: &Value) -> Result<(), String> {
        let id = number(value, "id")?;
        let revision = number(value, "revision")?;
        let actual = text(value, "text")?;
        let attempt = &mut self.attempts[index];
        require(attempt.sent, "response acknowledges an incompletely sent request")?;
        require(id != 0usize && revision != 0usize, "zero acknowledged id/revision")?;
        require(attempt.id == 0usize || id == attempt.id, "acknowledged wrong replacement id")?;
        require(actual == attempt.text, "acknowledgement has incomplete or changed text")?;
        require(attempt.acknowledged_revision == 0usize, "duplicate acknowledgement")?;
        attempt.acknowledged_id = id;
        attempt.acknowledged_revision = revision;
        self.acknowledgements += 1usize;
        Ok(())
    }
    pub fn disk(&mut self, root: &mut Directory) -> Result<Snapshot, String> {
        let mut file = io(root.open("journal.log"), "oracle open log")?;
        let mut entries: Vec<Entry> = Vec::new();
        let mut revision = 0usize;
        let mut offset = 0usize;
        let mut seen = vec![false; self.attempts.len()];
        let mut acknowledged = 0usize;
        let mut last_digest = [0u8; 32];
        loop {
            let mut header = [0u8; 40];
            let count = read_part(&mut file, &mut header)?;
            if count == 0usize { break; }
            require(count == 40usize, "partial header visible after recovery")?;
            require(&header[..8] == b"LKJLOG01", "oracle log magic")?;
            let length = word(&header, 8usize);
            let id = word(&header, 16usize);
            let actual_revision = word(&header, 24usize);
            let operation = word(&header, 32usize);
            require(actual_revision == revision + 1usize, "oracle revision gap or duplicate")?;
            let mut bytes: Vec<u8> = Vec::new();
            let mut left = length;
            let mut hash = Sha256::new();
            hash.update(&header);
            while left != 0usize {
                let mut chunk = [0u8; 8192];
                let size = if left < chunk.len() { left } else { chunk.len() };
                require(read_part(&mut file, &mut chunk[..size])? == size, "partial text visible after recovery")?;
                hash.update(&chunk[..size]);
                bytes.extend_from_slice(&chunk[..size]);
                left -= size;
            }
            let mut digest = [0u8; 32];
            require(read_part(&mut file, &mut digest)? == 32usize, "partial digest visible after recovery")?;
            require(hash.finish() == digest, "oracle log digest mismatch")?;
            let actual = match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => return Err("oracle invalid UTF-8".to_string()),
            };
            let sequence = serial(&actual)?;
            require(sequence != 0usize && sequence <= self.attempts.len(), "unknown operation in journal")?;
            let index = sequence - 1usize;
            require(!seen[index], "same issued mutation appeared twice")?;
            seen[index] = true;
            let attempt = &self.attempts[index];
            require(attempt.sent && actual == attempt.text, "unsent or incomplete mutation became visible")?;
            if attempt.id == 0usize {
                require(operation == 1usize && id == entries.len() + 1usize, "invalid appended id")?;
            } else {
                require(operation == 2usize && id == attempt.id && id <= entries.len(), "invalid replaced id")?;
            }
            if attempt.acknowledged_revision != 0usize {
                require(attempt.acknowledged_revision == actual_revision
                    && attempt.acknowledged_id == id, "acknowledged history changed")?;
                acknowledged += 1usize;
            }
            let entry = Entry { id, revision: actual_revision, text: actual };
            if operation == 1usize { entries.push(entry); } else { entries[id - 1usize] = entry; }
            revision = actual_revision;
            offset += length + 72usize;
            last_digest = digest;
        }
        require(acknowledged == self.acknowledgements,
            &format!("lost acknowledged history: found {acknowledged} of {}", self.acknowledgements))?;
        let mut head_file = io(root.open("head"), "oracle open head")?;
        let mut head = [0u8; 97];
        require(read_part(&mut head_file, &mut head)? == 96usize, "head framing after recovery")?;
        require(&head[..8] == b"LKJHEAD1", "head magic after recovery")?;
        require(word(&head, 8usize) == offset && word(&head, 16usize) == revision
            && word(&head, 24usize) == entries.len() + 1usize, "head differs from complete log")?;
        require(&head[32..64] == &last_digest[..], "head final-record digest")?;
        require(&std::sha256::digest(&head[..64])[..] == &head[64..96], "head checksum")?;
        self.verified_frames += revision;
        self.snapshots += 1usize;
        Ok(Snapshot { entries, revision, bytes: offset })
    }
}
pub fn entry(actual: &Value, expected: &Entry) -> Result<(), String> {
    require(number(actual, "id")? == expected.id, "HTTP id differs from durable log")?;
    require(number(actual, "revision")? == expected.revision, "HTTP revision differs from durable log")?;
    require(text(actual, "text")? == expected.text, "HTTP text differs from complete durable record")
}
pub fn all(actual: &Value, snapshot: &Snapshot) -> Result<(), String> {
    let values = actual.get("entries").ok_or("missing entries array".to_string())?
        .as_array().ok_or("entries is not an array".to_string())?;
    require(values.len() == snapshot.entries.len(), "HTTP entry count differs from log")?;
    for index in 0usize..values.len() { entry(&values[index], &snapshot.entries[index])?; }
    Ok(())
}
