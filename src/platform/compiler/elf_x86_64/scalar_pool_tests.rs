//! Execute native scalar and allocator routines with real mmap/munmap calls.
//! Passive observation overflow never grants or removes allocation permission.
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
    let fail = a.label();
    phase::process_context(&mut a);
    // Cached immutable scalars have no observable address identity.
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
            a.load(Cx, R14, 0);
            a.group(7, Cx, 0);
            a.when(NE, fail);
        }
    }
    for value in [256_i64, -2, i64::MIN, i64::MAX] {
        a.imm(Ax, value as u64);
        a.call(runtime.integer);
        a.mov(R13, Ax);
        a.load(Cx, R14, 0);
        a.group(7, Cx, 0);
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
    a.mov(Di, R14);
    a.call(runtime.dispose);
    a.group(7, Ax, 0);
    a.when(NE, fail);
    exit(&mut a, 0);
    a.bind(fail);
    exit(&mut a, 70);
    run_control(a, runtime, "scalar-pool")
}

#[test]
fn native_elf_phase_allocator_saturation_is_passive_and_disposal_resets_exact_state()
-> std::result::Result<(), String> {
    let mut a = Asm::new();
    let runtime = Runtime::new(&mut a);
    let fail = a.label();
    // A caller-owned Handle160 and a transient 128-byte recovery frame.
    a.group(5, Sp, 288);
    a.mov(R14, Sp);
    a.lea(R15, Sp, 160);
    a.imm(Ax, 0);
    for offset in (0..288).step_by(8) {
        a.store(Sp, offset, Ax);
    }
    a.store(R15, 0, Sp);
    a.address(Ax, fail);
    a.store(R15, 8, Ax);
    a.store(R15, 24, R14);
    a.imm(Ax, u64::MAX);
    a.store(R14, 144, Ax);
    // Attempt overflow observes a real allocation; it never denies it.
    a.imm(Di, 0);
    a.call(runtime.alloc);
    a.group(7, Ax, 0);
    a.when(E, fail);
    a.mov(R12, Ax);
    a.imm(Cx, 0x1234);
    a.store(R12, 0, Cx);
    a.load(Cx, R14, 152);
    a.group(7, Cx, 1);
    a.when(NE, fail);
    // Seed only observations, then force a second real mapping.
    a.imm(Ax, u64::MAX);
    a.store(R14, 24, Ax);
    a.store(R14, 32, Ax);
    a.imm(Di, 65536);
    a.call(runtime.alloc);
    a.mov(Cx, Ax);
    a.group(4, Cx, 63);
    a.when(NE, fail);
    for offset in [24, 32, 144] {
        a.load(Cx, R14, offset);
        a.imm(Dx, u64::MAX);
        a.bin(0x39, Cx, Dx);
        a.when(NE, fail);
    }
    a.load(Cx, R14, 40);
    a.group(7, Cx, 3);
    a.when(NE, fail);
    a.load(Cx, R12, 0);
    a.group(7, Cx, 0x1234);
    a.when(NE, fail);
    // Links identify payload starts, so base zero is not the empty sentinel.
    a.load(Cx, R14, 0);
    a.load(Cx, Cx, -48);
    a.bin(0x39, Cx, R14);
    a.when(NE, fail);
    a.mov(Di, R14);
    a.call(runtime.dispose);
    a.group(7, Ax, 0);
    a.when(NE, fail);
    for offset in [0, 8, 16, 24, 32, 40, 56] {
        a.load(Cx, R14, offset);
        a.group(7, Cx, 0);
        a.when(NE, fail);
    }
    a.load(Cx, R14, 48);
    a.group(7, Cx, 2);
    a.when(NE, fail);
    // Reopening an empty context starts exact observations again.
    a.imm(Ax, 0);
    a.store(R14, 48, Ax);
    a.imm(Di, 16);
    a.call(runtime.alloc);
    a.load(Cx, R14, 24);
    a.group(7, Cx, 1);
    a.when(NE, fail);
    a.load(Cx, R14, 32);
    a.group(7, Cx, 65536);
    a.when(NE, fail);
    a.load(Cx, R14, 40);
    a.group(7, Cx, 0);
    a.when(NE, fail);
    // Checkpoint's call alignment must preserve the caller's flags too.
    a.imm(Ax, 17);
    a.group(7, Ax, 17);
    runtime.checkpoint(&mut a);
    a.when(NE, fail);
    a.mov(Di, R14);
    a.call(runtime.dispose);
    a.group(7, Ax, 0);
    a.when(NE, fail);
    exit(&mut a, 0);
    a.bind(fail);
    exit(&mut a, 70);
    run_control(a, runtime, "phase-saturation")
}

#[test]
fn native_elf_phase_internal_extent_boundaries_preserve_recovery_classes()
-> std::result::Result<(), String> {
    let mut a = Asm::new();
    let runtime = Runtime::new(&mut a);
    let fail = a.label();
    // Synthetic internal arithmetic controls: these do not claim a valid
    // source program can materialize an I64::MAX-sized Bytes or ByteBuffer.
    // Exercise the real helpers and recovery path without allocating memory.
    a.group(5, Sp, 288);
    a.mov(R14, Sp);
    a.lea(R15, Sp, 160);
    a.imm(Ax, 0);
    for offset in (0..288).step_by(8) {
        a.store(Sp, offset, Ax);
    }
    a.store(R15, 0, Sp);
    a.store(R15, 24, R14);
    for (case, label, status, attempts) in [
        (0, runtime.bytes_capacity, 2, 0),
        (1, runtime.buffer_capacity, 2, 0),
        (2, runtime.overflow, 3, 0),
        (3, runtime.bounds, 3, 0),
        (3, runtime.bytes_index, 3, 0),
        (3, runtime.buffer_index, 3, 0),
        (4, runtime.bytes_range, 3, 0),
        (5, runtime.capacity, 2, 1),
    ] {
        let recovered = a.label();
        a.address(Ax, recovered);
        a.store(R15, 8, Ax);
        a.imm(Ax, 0);
        a.store(R15, 64, Ax);
        a.store(R15, 72, Ax);
        match case {
            0 => {
                // An internal length outside the signed language domain.
                a.imm(Bx, u64::MAX);
                bytes::allocation_size(&mut a, label);
            }
            1 => {
                // The checked buffer successor fails before any allocation.
                a.imm(Bx, i64::MAX as u64);
                buffer::push_length(&mut a, label);
            }
            2 => {
                a.imm(Ax, i64::MAX as u64);
                a.group(0, Ax, 1);
                a.when(O, label);
            }
            3 => {
                // Unsigned bounds comparison rejects a negative index.
                a.imm(Ax, u64::MAX);
                a.imm(Cx, 0);
                a.bin(0x39, Ax, Cx);
                a.when(AE, label);
            }
            4 => {
                a.imm(Cx, u64::MAX);
                a.imm(Dx, 0);
                a.bin(0x39, Dx, Cx);
                a.when(B, label);
            }
            _ => {
                // Alignment overflows before mmap; the attempt is observed.
                a.imm(Di, i64::MAX as u64);
                a.call(runtime.alloc);
            }
        }
        a.jump(fail);
        a.bind(recovered);
        a.bin(0x39, Sp, R14);
        a.when(NE, fail);
        a.load(Cx, R15, 64);
        a.group(7, Cx, status);
        a.when(NE, fail);
        a.load(Cx, R15, 72);
        a.imm(Dx, label as u64);
        a.bin(0x39, Cx, Dx);
        a.when(NE, fail);
        // Neither context acquired a mapping or published an output root.
        for offset in (0..128).step_by(8) {
            a.load(Cx, R14, offset);
            a.group(7, Cx, 0);
            a.when(NE, fail);
        }
        a.load(Cx, R14, 144);
        a.group(7, Cx, attempts);
        a.when(NE, fail);
        a.load(Cx, R14, 152);
        a.group(7, Cx, 0);
        a.when(NE, fail);
    }
    exit(&mut a, 0);
    a.bind(fail);
    exit(&mut a, 70);
    run_control(a, runtime, "phase-internal-extent-boundaries")
}

fn exit(a: &mut Asm, status: u64) {
    a.imm(Di, status);
    a.imm(Ax, 60);
    a.syscall();
}

fn run_control(mut a: Asm, runtime: Runtime, name: &str) -> std::result::Result<(), String> {
    let mut pool = Vec::new();
    runtime.emit(&mut a, &mut pool);
    for (label, bytes) in pool {
        a.align();
        a.bind(label);
        a.bytes(&bytes);
    }
    let code = a.finish().map_err(|e| format!("assembler: {e:?}"))?;
    let bytes = elf(&code).map_err(|e| format!("ELF: {e:?}"))?;
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = directory.path().join(name);
    crate::platform::owned_output::publish_executable_create_new(
        &path,
        &bytes,
        None,
        "native scalar and phase allocator control",
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
                Err(format!("native {name} control failed: {status}"))
            };
        }
        if start.elapsed() >= Duration::from_secs(5) {
            return Err("explicit native test deadline exceeded".to_owned());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
