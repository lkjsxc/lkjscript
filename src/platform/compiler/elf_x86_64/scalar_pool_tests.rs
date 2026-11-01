//! Execute the real scalar routines and inspect their allocator cursor directly.
//! Interning immutable payloads is not a logical integer limit or reclamation.
use super::*;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Joined(Child);
impl Drop for Joined {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn native_elf_scalar_pool_preserves_payloads_without_advancing_the_heap()
-> std::result::Result<(), String> {
    let mut a = Asm::new();
    let runtime = Runtime::new(&mut a);
    let mut pool = Vec::new();
    let fail = a.label();
    a.imm(R14, 0);
    a.imm(R15, 0);
    // Every cached integer and Boolean must return a tagged immutable payload
    // without allocating, even across interleaved scalar kinds and repetition.
    for (label, tag, values) in [
        (runtime.integer, 2_u64, (-1_i64..=255).collect::<Vec<_>>()),
        (runtime.boolean, 1, vec![0, 1, 0, 1]),
        (runtime.integer, 2, vec![-1, 0, 1, 255, 128]),
    ] {
        for value in values {
            a.imm(Ax, value as u64);
            a.call(label);
            a.load(Cx, Ax, 0);
            a.imm(Dx, tag);
            a.bin(0x39, Cx, Dx);
            a.when(NE, fail);
            a.load(Cx, Ax, 8);
            a.imm(Dx, value as u64);
            a.bin(0x39, Cx, Dx);
            a.when(NE, fail);
            a.group(7, R15, 0);
            a.when(NE, fail);
            a.group(7, R14, 0);
            a.when(NE, fail);
        }
    }
    // Outside the optimization's physical table every I64 remains supported by
    // the ordinary allocator. Retaining one result must survive later calls.
    for value in [256_i64, -2, i64::MIN, i64::MAX] {
        a.imm(Ax, value as u64);
        a.call(runtime.integer);
        a.mov(R13, Ax);
        a.group(7, R15, 0);
        a.when(E, fail);
        for cached in [0_i64, 255, -1] {
            a.imm(Ax, cached as u64);
            a.call(runtime.integer);
        }
        a.imm(Ax, 1);
        a.call(runtime.boolean);
        a.load(Cx, R13, 0);
        a.group(7, Cx, 2);
        a.when(NE, fail);
        a.load(Cx, R13, 8);
        a.imm(Dx, value as u64);
        a.bin(0x39, Cx, Dx);
        a.when(NE, fail);
    }
    a.imm(Di, 0);
    a.imm(Ax, 60);
    a.syscall();
    a.bind(fail);
    a.imm(Di, 70);
    a.imm(Ax, 60);
    a.syscall();
    runtime.emit(&mut a, &mut pool);
    for (label, bytes) in pool {
        a.align();
        a.bind(label);
        a.bytes(&bytes);
    }
    let code = a.finish().map_err(|e| format!("assembler: {e:?}"))?;
    let bytes = elf(&code).map_err(|e| format!("ELF: {e:?}"))?;
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = directory.path().join("scalar-pool");
    crate::platform::owned_output::publish_executable_create_new(
        &path,
        &bytes,
        None,
        "scalar pool native control",
    )
    .map_err(|e| format!("publication: {e:?}"))?;
    let mut child = Joined(
        Command::new(path)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?,
    );
    let start = Instant::now();
    loop {
        if let Some(status) = child.0.try_wait().map_err(|e| e.to_string())? {
            return if status.code() == Some(0) {
                Ok(())
            } else {
                Err(format!(
                    "scalar payload/allocation control failed: {status}"
                ))
            };
        }
        if start.elapsed() >= Duration::from_secs(5) {
            return Err("explicit native test deadline exceeded".to_owned());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
