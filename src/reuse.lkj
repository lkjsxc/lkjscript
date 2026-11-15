//! Pure identities and authenticated reuse. The driver owns all store I/O.
//! Cache readers do not acquire proof authority from filenames or checksums.
use crate::{ir, ir_binary, sha256};
use std::collections::BTreeMap;

fn hex(bytes: &[u8]) -> String {
    let alphabet = b"0123456789abcdef";
    let mut text = String::new();
    for byte in bytes {
        text.push(alphabet[(*byte >> 4u8) as usize] as char);
        text.push(alphabet[(*byte & 15u8) as usize] as char);
    }
    text
}

#[derive(Clone)]
pub struct Keys {
    graph: String,
    functions: Vec<String>,
    program: String,
}
impl Keys {
    /// Body checking reads the type/destructor graph and explicit callee
    /// contracts, never a callee's body. Validate that graph on every use.
    pub fn new(program: &ir::Program) -> Self {
        let mut signatures = Vec::new();
        for function in &program.functions {
            let mut parameters = Vec::new();
            for local in function.locals.iter().take(function.params) {
                parameters.push(local.clone());
            }
            signatures.push(ir::Function {
                file: function.file,
                name: function.name.clone(),
                params: function.params,
                result: function.result,
                return_source: function.return_source.clone(),
                locals: parameters,
                body: Vec::new(),
                test: function.test,
            });
        }
        let shape = ir::Program {
            types: program.types.clone(),
            functions: signatures,
            destructors: program.destructors.clone(),
        };
        let encoded = ir_binary::write(&shape);
        let mut hash = sha256::Sha256::new();
        hash.update(b"lkjscript-reuse-graph-1\0");
        hash.update(&encoded);
        // Preserve the explicit return-contract domain in graph identities.
        hash.update(b"\0return-contract-tags\0");
        for function in &program.functions {
            let kind = match &function.return_source {
                None => 0u8,
                Some(ir::ReturnSource::Parameter { .. }) => 1u8,
                Some(ir::ReturnSource::Static) => 2u8,
                Some(ir::ReturnSource::Leaves(_)) => 3u8,
            };
            hash.update(&[kind]);
        }
        let graph = hash.finish();
        let mut complete = sha256::Sha256::new();
        complete.update(b"lkjscript-reuse-program-1\0");
        complete.update(&graph);
        let mut functions = Vec::new();
        for function in &program.functions {
            let text = ir_binary::write_function(function);
            let mut hash = sha256::Sha256::new();
            hash.update(b"lkjscript-reuse-function-1\0");
            hash.update(&graph);
            hash.update(&text);
            let digest = hash.finish();
            complete.update(&digest);
            functions.push(hex(&digest));
        }
        Self { graph: hex(&graph), functions, program: hex(&complete.finish()) }
    }
    pub fn graph(&self) -> &str { &self.graph }
    pub fn function(&self, index: usize) -> &str { &self.functions[index] }
    pub fn all(&self) -> &[String] { &self.functions }
    pub fn program(&self) -> &str { &self.program }
}

/// Exact, allocation-free counterpart of Keys' signature graph. Both inputs
/// have passed structure validation. Ordinary body checking reads these shared
/// inputs and explicit callee contracts, never another function's body.
pub(crate) fn same_check_graph(left: &ir::Program, right: &ir::Program) -> bool {
    if left.types != right.types || left.destructors != right.destructors
        || left.functions.len() != right.functions.len() {
        return false;
    }
    for (left, right) in left.functions.iter().zip(right.functions.iter()) {
        if left.file != right.file || left.name != right.name || left.test != right.test
            || left.params != right.params || left.result != right.result
            || left.return_source != right.return_source
            || left.locals[..left.params] != right.locals[..right.params] {
            return false;
        }
    }
    true
}

/// The exact program is inseparable from the identities computed for it.
/// A producer cannot substitute keys from a different, already checked body.
pub struct Prepared {
    program: ir::Program,
    keys: Keys,
}
impl Prepared {
    pub fn new(program: ir::Program) -> Self {
        let keys = Keys::new(&program);
        Self { program, keys }
    }
    pub fn keys(&self) -> &Keys { &self.keys }
    pub(crate) fn into_parts(self) -> (ir::Program, Keys) { (self.program, self.keys) }
}

/// Only the checker creates a cached pair. Normal checked callers need not hash.
pub struct Checked {
    checked: ir::CheckedProgram,
    keys: Option<Keys>,
}
impl Checked {
    pub fn uncached(checked: ir::CheckedProgram) -> Self {
        Self { checked, keys: None }
    }
    pub(crate) fn cached(checked: ir::CheckedProgram, keys: Keys) -> Self {
        Self { checked, keys: Some(keys) }
    }
    pub fn program(&self) -> &ir::Program { self.checked.program() }
    pub fn checked(&self) -> &ir::CheckedProgram { &self.checked }
    pub fn keys(&self) -> Option<&Keys> { self.keys.as_ref() }
}

/// Retain the original oracle while selecting only separately checked output.
/// Rejected optimization never changes the original program or its cache keys.
pub struct Selection {
    original: Checked,
    optimized: Option<Checked>,
    rejected: bool,
}
impl Selection {
    pub fn unchanged(original: Checked) -> Self {
        Self { original, optimized: None, rejected: false }
    }
    pub fn after_check(
        original: Checked, candidate: Result<Checked, Vec<crate::diagnostic::Diagnostic>>,
    ) -> Self {
        match candidate {
            Ok(checked) => Self { original, optimized: Some(checked), rejected: false },
            Err(_) => Self { original, optimized: None, rejected: true },
        }
    }
    pub fn original(&self) -> &Checked { &self.original }
    pub fn native(&self) -> &Checked {
        match &self.optimized { Some(checked) => checked, None => &self.original }
    }
    pub fn optimized(&self) -> bool { self.optimized.is_some() }
    pub fn rejected(&self) -> bool { self.rejected }
}

struct Hmac {
    inner: sha256::Sha256,
    outer: [u8; 64],
}
impl Hmac {
    fn new(secret: &[u8]) -> Self {
        let key = if secret.len() > 64 {
            let digest = sha256::digest(secret);
            digest[..].to_vec()
        } else { secret.to_vec() };
        let mut inner = [0x36u8; 64];
        let mut outer = [0x5cu8; 64];
        for index in 0..key.len() {
            inner[index] ^= key[index];
            outer[index] ^= key[index];
        }
        let mut hash = sha256::Sha256::new();
        hash.update(&inner);
        Self { inner: hash, outer }
    }
    fn update(&mut self, bytes: &[u8]) { self.inner.update(bytes); }
    fn length(&mut self, size: usize) {
        let mut length = [0u8; 8];
        for index in 0..8 {
            length[index] = ((size as u64) >> (index * 8)) as u8;
        }
        self.update(&length);
    }
    fn field(&mut self, bytes: &[u8]) {
        self.length(bytes.len());
        self.update(bytes);
    }
    fn finish(self) -> [u8; 32] {
        let inside = self.inner.finish();
        let mut outside = sha256::Sha256::new();
        outside.update(&self.outer);
        outside.update(&inside);
        outside.finish()
    }
}
pub fn hmac(secret: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hash = Hmac::new(secret);
    hash.update(bytes);
    hash.finish()
}

fn start_tag(producer: &[u8], secret: &[u8], namespace: &str, key: &str, length: usize) -> Hmac {
    let mut mac = Hmac::new(secret);
    mac.field(b"lkjscript-authenticated-store-1");
    mac.field(producer);
    mac.field(b"linux-x86_64");
    mac.field(namespace.as_bytes());
    mac.field(key.as_bytes());
    mac.length(length);
    mac
}
fn tag(producer: &[u8], secret: &[u8], namespace: &str, key: &str, payload: &[u8]) -> [u8; 32] {
    let mut mac = start_tag(producer, secret, namespace, key, payload.len());
    mac.update(payload);
    mac.finish()
}
fn magic() -> &'static [u8] { b"lkjscript-store-1\n" }

pub fn header_size() -> usize { magic().len() + 8 }
pub fn payload_length(header: &[u8]) -> Option<usize> {
    if header.len() != header_size() || &header[..magic().len()] != magic() { return None; }
    let mut length = 0u64;
    for index in 0..8 {
        length |= (header[magic().len() + index] as u64) << (index * 8);
    }
    let size = length as usize;
    if size as u64 != length { return None; }
    Some(size)
}

/// Authenticate a large disposable entry using constant-size I/O buffers before
/// allocating its payload. A host must also require EOF and authenticate again
/// after a bounded reread if another writer can change the same file.
pub struct Verifier {
    mac: Hmac,
    remaining: usize,
    valid: bool,
}
impl Verifier {
    pub fn new(producer: &[u8], secret: &[u8], namespace: &str, key: &str, length: usize) -> Option<Self> {
        if producer.len() != 32 || secret.len() != 32 { return None; }
        Some(Self { mac: start_tag(producer, secret, namespace, key, length), remaining: length, valid: true })
    }
    pub fn update(&mut self, bytes: &[u8]) -> bool {
        if !self.valid || bytes.len() > self.remaining {
            self.valid = false;
            return false;
        }
        self.mac.update(bytes);
        self.remaining -= bytes.len();
        true
    }
    pub fn finish(self, supplied: &[u8]) -> bool {
        if !self.valid || self.remaining != 0 || supplied.len() != 32 { return false; }
        let expected = self.mac.finish();
        let mut different = 0u8;
        for index in 0..32 { different |= expected[index] ^ supplied[index]; }
        different == 0
    }
}

/// A key is separate trusted authority; it must never reside in the store or a
/// source Directory granted to an untrusted producer. Both identities are 32 bytes.
pub fn seal(
    producer: &[u8], secret: &[u8], namespace: &str, key: &str, payload: &[u8],
) -> Vec<u8> {
    if producer.len() != 32 || secret.len() != 32 { return Vec::new(); }
    let mut bytes = magic().to_vec();
    for index in 0..8 {
        bytes.push(((payload.len() as u64) >> (index * 8)) as u8);
    }
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(&tag(producer, secret, namespace, key, payload));
    bytes
}

/// Verify exact framing and the MAC before allocating or interpreting payload
/// fields. A replay for identical inputs is harmless; another identity rejects.
pub fn authenticate(
    producer: &[u8], secret: &[u8], namespace: &str, key: &str, bytes: &[u8],
) -> Option<Vec<u8>> {
    if producer.len() != 32 || secret.len() != 32 { return None; }
    let header = magic().len() + 8;
    if bytes.len() < header + 32 || &bytes[..magic().len()] != magic() { return None; }
    let mut length = 0u64;
    for index in 0..8 {
        length |= (bytes[magic().len() + index] as u64) << (index * 8);
    }
    let actual = bytes.len() - header - 32;
    if length != actual as u64 { return None; }
    let payload = &bytes[header..header + actual];
    let expected = tag(producer, secret, namespace, key, payload);
    let mut different = 0u8;
    for index in 0..32 {
        different |= expected[index] ^ bytes[header + actual + index];
    }
    if different != 0 { return None; }
    Some(payload.to_vec())
}

#[derive(Clone)]
pub struct Pending {
    pub namespace: String,
    pub key: String,
    pub bytes: Vec<u8>,
}
pub struct Records {
    producer: Vec<u8>,
    secret: Vec<u8>,
    entries: BTreeMap<String, Vec<u8>>,
    pending: Vec<Pending>,
    packed: Option<(String, String)>,
    dirty: bool,
    pub hits: usize,
    pub misses: usize,
    pub rejected: usize,
    pub writes: usize,
}
fn identity(namespace: &str, key: &str) -> String {
    format!("{}:{namespace}{key}", namespace.len())
}
impl Records {
    pub fn disabled() -> Self { Self::new(b"", b"") }
    pub fn new(producer: &[u8], secret: &[u8]) -> Self {
        Self {
            producer: producer.to_vec(), secret: secret.to_vec(),
            entries: BTreeMap::new(), pending: Vec::new(), packed: None, dirty: false,
            hits: 0, misses: 0, rejected: 0, writes: 0,
        }
    }
    pub fn packed(producer: &[u8], secret: &[u8], namespace: &str, group: &str) -> Self {
        let mut records = Self::new(producer, secret);
        records.packed = Some((namespace.to_string(), group.to_string()));
        records
    }
    /// One independently authenticated collection avoids a filesystem object
    /// and an HMAC initialization for every tiny body proof or code fragment.
    /// The envelope still binds exact producer, target, namespace and graph.
    pub fn load_pack(&mut self, envelope: &[u8]) -> bool {
        let Some((namespace, group)) = &self.packed else { return false; };
        let Some(payload) = authenticate(&self.producer, &self.secret, namespace, group, envelope) else {
            self.rejected += 1; return false;
        };
        let Some(entries) = decode_pack(namespace, &payload) else {
            self.rejected += 1; return false;
        };
        self.entries = entries;
        true
    }
    /// Keep one graph generation's requested bodies. Evicted old bodies are
    /// misses, and a stream of unique mutants cannot grow one pack quadratically.
    pub fn retain(&mut self, namespace: &str, keys: &[String]) {
        let mut retained = BTreeMap::new();
        for key in keys {
            let id = identity(namespace, key);
            if let Some(value) = self.entries.remove(&id) { retained.insert(id, value); }
        }
        self.entries = retained;
    }
    pub fn sealed_pack(&self) -> Option<Pending> {
        if !self.dirty { return None; }
        let (namespace, group) = self.packed.as_ref()?;
        let mut payload = b"LKJPACK1"[..].to_vec();
        pack_word(&mut payload, self.entries.len());
        for (key, value) in &self.entries {
            pack_bytes(&mut payload, key.as_bytes());
            pack_bytes(&mut payload, value);
        }
        Some(Pending {
            namespace: namespace.clone(), key: group.clone(),
            bytes: seal(&self.producer, &self.secret, namespace, group, &payload),
        })
    }
    pub fn enabled(&self) -> bool
 { self.producer.len() == 32 && self.secret.len() == 32 }
    pub fn load(&mut self, namespace: &str, key: &str, bytes: &[u8]) -> bool {
        if !self.enabled() { return false; }
        match authenticate(&self.producer, &self.secret, namespace, key, bytes) {
            Some(payload) => {
                self.entries.insert(identity(namespace, key), payload);
                true
            }
            None => { self.rejected += 1; false }
        }
    }
    pub fn get(&mut self, namespace: &str, key: &str) -> Option<Vec<u8>> {
        if !self.enabled() { return None; }
        match self.entries.get(&identity(namespace, key)) {
            Some(payload) => { self.hits += 1; Some(payload.clone()) }
            None => { self.misses += 1; None }
        }
    }
    pub fn put(&mut self, namespace: &str, key: &str, payload: &[u8]) {
        if !self.enabled() { return; }
        let identity = identity(namespace, key);
        if let Some(previous) = self.entries.get(&identity) {
            if previous.as_slice() == payload { return; }
        }
        if let Some((owner, _)) = &self.packed {
            if owner != namespace { return; }
            self.entries.insert(identity, payload.to_vec());
            self.dirty = true;
        } else {
            let bytes = seal(&self.producer, &self.secret, namespace, key, payload);
            self.entries.insert(identity, payload.to_vec());
            self.pending.push(Pending {
                namespace: namespace.to_string(), key: key.to_string(), bytes,
            });
        }
        self.writes += 1;
    }
    pub fn pending(&self) -> &[Pending] { &self.pending }
}

fn pack_word(out: &mut Vec<u8>, value: usize) {
    for shift in 0..8 { out.push((value >> (shift * 8)) as u8); }
}
fn pack_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    pack_word(out, bytes.len()); out.extend_from_slice(bytes);
}
fn unpack_word(bytes: &[u8], at: &mut usize) -> Option<usize> {
    if bytes.len() - *at < 8 { return None; }
    let mut result = 0usize;
    for shift in 0..8 { result |= (bytes[*at + shift] as usize) << (shift * 8); }
    *at += 8; Some(result)
}
fn unpack_bytes(bytes: &[u8], at: &mut usize) -> Option<Vec<u8>> {
    let length = unpack_word(bytes, at)?;
    if length > bytes.len() - *at { return None; }
    let result = bytes[*at..*at + length].to_vec(); *at += length; Some(result)
}
fn decode_pack(namespace: &str, bytes: &[u8]) -> Option<BTreeMap<String, Vec<u8>>> {
    if bytes.len() < 16 || &bytes[..8] != b"LKJPACK1" { return None; }
    let mut at = 8usize;
    let count = unpack_word(bytes, &mut at)?;
    if count > (bytes.len() - at) / 16 { return None; }
    let prefix = format!("{}:{namespace}", namespace.len());
    let mut entries = BTreeMap::new();
    for _ in 0..count {
        let key = String::from_utf8(unpack_bytes(bytes, &mut at)?).ok()?;
        if !key.starts_with(&prefix) || entries.contains_key(&key) { return None; }
        let value = unpack_bytes(bytes, &mut at)?;
        entries.insert(key, value);
    }
    if at == bytes.len() { Some(entries) } else { None }
}
