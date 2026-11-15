//! Reuse only execution of the exact fresh, closed gate image.
//! The trusted gate script checks/builds one IR snapshot first. A matching digest
//! catches stale/swapped reports, not a dishonest key holder. This is not an IR verdict:
//! its HMAC producer is this runner, and its identity includes all artifact bytes.
pub mod diagnostic;
pub mod ir;
pub mod ir_binary;
pub mod reuse;
pub mod store;
pub use std::sha256;

fn io(operation: &str, errno: i64) -> String {
    format!("E_GATE_REUSE {operation} failed (errno {errno})")
}
fn read(world: &mut World, path: &str) -> Result<Vec<u8>, String> {
    let (parent, name) = match path.rfind('/') {
        Some(0) => ("/", &path[1..]),
        Some(at) => (&path[..at], &path[at + 1..]),
        None => (".", path),
    };
    let mut directory = world.directory(parent).map_err(|errno: i64| io("directory", errno))?;
    let mut file = directory.open(name).map_err(|errno: i64| io("open", errno))?;
    file.read_to_end().map_err(|errno: i64| io("read", errno))
}
fn temporary(world: &mut World) -> Result<File, String> {
    world.temporary_file().map_err(|errno: i64| io("temporary file", errno))
}
fn captured(file: &mut File) -> Result<Vec<u8>, String> {
    file.rewind().map_err(|errno: i64| io("rewind capture", errno))?;
    file.read_to_end().map_err(|errno: i64| io("read capture", errno))
}
fn execute(world: &mut World, bytes: &[u8]) -> Result<(), String> {
    // Execute the same bytes we hashed, never reopen an artifact pathname.
    let mut storage = temporary(world)?;
    storage.write_all(bytes).map_err(|errno: i64| io("write image", errno))?;
    storage.set_executable().map_err(|errno: i64| io("image executable", errno))?;
    let executable = storage.readonly().map_err(|errno: i64| io("freeze image", errno))?;
    drop(storage);
    let input = temporary(world)?;
    let mut output = temporary(world)?;
    let mut errors = temporary(world)?;
    let mut result = temporary(world)?;
    let arguments: Vec<Vec<u8>> = Vec::new();
    let mut child = world.process().spawn_file(&executable, &arguments, &input, &output, &errors, &result)
        .map_err(|errno: i64| io("spawn closed image", errno))?;
    let status = match child.wait() {
        Ok(status) => status,
        Err(errno) => {
            let _ = child.terminate();
            let _ = child.wait();
            return Err(io("wait closed image", errno));
        }
    };
    let output = captured(&mut output)?;
    let errors = captured(&mut errors)?;
    let result = captured(&mut result)?;
    if status != 0 || !output.is_empty() || !errors.is_empty() || !result.is_empty() {
        if !errors.is_empty() { world.stderr().write(&errors); }
        return Err(format!("E_GATE_REUSE closed execution: status={status} stdout={} stderr={} result={}",
            output.len(), errors.len(), result.len()));
    }
    Ok(())
}
fn run(world: &mut World) -> Result<(), String> {
    let raw = world.args();
    if raw.len() != 6usize {
        return Err("E_GATE_REUSE expected ELF AUTHORITY GATE_SCRIPT CASE_COUNT STORE KEY".to_string());
    }
    let mut args = Vec::new();
    for argument in raw {
        args.push(String::from_utf8(argument).map_err(|_| -> String { "E_GATE_REUSE argument is not UTF-8".to_string() })?);
    }
    let count = args[3].as_str().parse::<usize>().map_err(|_| -> String { "E_GATE_REUSE invalid case count".to_string() })?;
    if count == 0 { return Err("E_GATE_REUSE empty case group".to_string()); }
    let bytes = read(world, &args[0])?;
    let authority = read(world, &args[1])?;
    let closed = b"entry main\ndeclared authority:\n  none\nreachable operations (conservative checked-IR branches and owned cleanup):\n  none\n";
    if !authority.starts_with(closed) {
        return Err("E_GATE_REUSE authority report must have no grants or reachable effects".to_string());
    }
    let footer = format!("closed-image-sha256 {}\n", sha256::hex_digest(&bytes));
    let mut matching = authority.len() >= footer.len();
    if matching {
        let start = authority.len() - footer.len();
        for index in 0usize..footer.len() {
            if authority[start + index] != footer.as_bytes()[index] { matching = false; }
        }
    }
    if !matching {
        return Err("E_GATE_REUSE authority report does not end with the exact image digest".to_string());
    }
    let script = read(world, &args[2])?;
    let mut identity = b"lkjscript-closed-gate-1\0"[..].to_vec();
    store::push_bytes(&mut identity, &bytes);
    store::push_bytes(&mut identity, &authority);
    store::push_bytes(&mut identity, &script);
    store::push_word(&mut identity, count);
    // The runner fixes argv/stdin, normal exit, and all three capture expectations.
    store::push_bytes(&mut identity, b"argv=[];stdin=[];status=0;stdout=[];stderr=[];result=[]");
    let key = sha256::hex_digest(&identity);
    let mut cache = store::Store::open(world, &Some(args[4].clone()), &Some(args[5].clone()));
    let success = b"closed execution succeeded\0"[..].to_vec();
    let mut reused = false;
    if let Some(cache) = &mut cache {
        if let Some(payload) = cache.get("gate-closed", &key) { reused = payload == success; }
    }
    if !reused {
        execute(world, &bytes)?;
        if let Some(cache) = &mut cache {
            cache.put("gate-closed", &key, &success);
            cache.finish();
        }
    }
    let skipped = if reused { count } else { 0usize };
    let rerun = if reused { 0usize } else { count };
    world.stdout().write(format!("LKJSCRIPT_CLOSED_GATE reused={skipped} rerun={rerun} proof=authenticated-exact-runner-sealed-ELF-and-input-contract\n").as_bytes());
    Ok(())
}
pub fn main(mut world: World) {
    if let Err(error) = run(&mut world) {
        world.stderr().write(format!("{error}\n").as_bytes());
        world.set_exit_status(1u8);
    }
}
