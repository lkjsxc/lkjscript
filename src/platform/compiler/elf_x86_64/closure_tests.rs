//! Execute the production callable helpers against independent frame expectations.
//! These controls deliberately constrain test stack/deadline resources, never
//! production admission. No source-level address identity is exposed.
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

fn exit(a: &mut Asm, status: u64) {
    a.imm(Di, status);
    a.imm(Ax, 60);
    a.syscall();
}

fn require(a: &mut Asm, fail: Label, actual: R, expected: u64) {
    a.imm(R11, expected);
    a.bin(0x39, actual, R11);
    a.when(NE, fail);
}

fn run(mut a: Asm, runtime: Runtime, name: &str) -> std::result::Result<(), String> {
    let mut pool = Vec::new();
    runtime.emit(&mut a, &mut pool);
    for (label, bytes) in pool {
        a.align();
        a.bind(label);
        a.bytes(&bytes);
    }
    let code = a.finish().map_err(|e| format!("assembler: {e:?}"))?;
    let image = elf(&code).map_err(|e| format!("ELF: {e:?}"))?;
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    let binary = directory.path().join(name);
    crate::platform::owned_output::publish_executable_create_new(
        &binary,
        &image,
        None,
        "native callable frame control",
    )
    .map_err(|e| format!("publish: {e:?}"))?;
    let mut child = Joined(
        Command::new("/usr/bin/prlimit")
            .env_clear()
            .args(["--stack=262144", "--core=0", "--"])
            .arg(binary)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?,
    );
    let started = Instant::now();
    loop {
        if let Some(status) = child.0.try_wait().map_err(|e| e.to_string())? {
            return if status.code() == Some(0) {
                Ok(())
            } else {
                Err(format!("{name}: native status {status}"))
            };
        }
        if started.elapsed() >= Duration::from_secs(5) {
            return Err(format!("{name}: explicit test deadline exceeded"));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn native_elf_empty_bind_is_identity_without_any_allocation() -> std::result::Result<(), String> {
    let mut a = Asm::new();
    let runtime = Runtime::new(&mut a);
    let fail = a.label();
    let unbound = a.label();
    let bound = a.label();
    let metadata = a.label();
    let target = a.label();
    phase::process_context(&mut a);
    for callable in [unbound, bound] {
        a.address(Ax, callable);
        a.push(Ax);
        a.mov(R12, Sp);
        closures::bind(&mut a, &runtime, 0);
        a.bin(0x39, Sp, R12);
        a.when(NE, fail);
        a.load(Ax, Sp, 0);
        a.address(Cx, callable);
        a.bin(0x39, Ax, Cx);
        a.when(NE, fail);
        a.group(0, Sp, 8);
        // Context64 includes the slab head, cursor, extent and all observations.
        for offset in (0..64).step_by(8) {
            a.load(Ax, R14, offset);
            require(&mut a, fail, Ax, 0);
        }
        require(&mut a, fail, R15, 0);
    }
    a.address(Ax, bound);
    a.load(Cx, Ax, 16);
    require(&mut a, fail, Cx, 2);
    a.load(Cx, Ax, 24);
    require(&mut a, fail, Cx, 73);
    a.load(Cx, Ax, 32);
    require(&mut a, fail, Cx, 91);
    exit(&mut a, 0);
    a.bind(fail);
    exit(&mut a, 70);
    a.bind(target);
    a.ret();
    a.align();
    a.bind(unbound);
    a.bytes(&6_u64.to_le_bytes());
    a.absolute(metadata);
    a.bytes(&0_u64.to_le_bytes());
    a.bind(bound);
    a.bytes(&6_u64.to_le_bytes());
    a.absolute(metadata);
    a.bytes(&2_u64.to_le_bytes());
    a.bytes(&73_u64.to_le_bytes());
    a.bytes(&91_u64.to_le_bytes());
    a.bind(metadata);
    a.absolute(target);
    a.bytes(&0_u64.to_le_bytes());
    a.bytes(&3_u64.to_le_bytes());
    a.bytes(&0_u64.to_le_bytes());
    run(a, runtime, "empty-bind")
}

#[test]
fn native_elf_tail_frame_preserves_overlapping_arguments_and_original_continuation()
-> std::result::Result<(), String> {
    // Both nontrivial overlap directions, shrinking frames, complete binding,
    // and a zero-arity/zero-local body are independently checked.
    for (old_locals, captured, arguments, locals) in [
        (0_u32, 0_u32, 4_u32, 5_u32),
        (0, 3, 4, 8),
        (9, 0, 1, 1),
        (2, 3, 0, 7),
        (0, 0, 0, 0),
    ] {
        let mut a = Asm::new();
        let runtime = Runtime::new(&mut a);
        let fail = a.label();
        let entry = a.label();
        let body = a.label();
        let prefix = a.label();
        let saved = [
            (Bx, 0x11_2233_4455_6677_u64),
            (R12, 0x22_3344_5566_7788),
            (R13, 0x33_4455_6677_8899),
        ];
        phase::process_context(&mut a);
        for (register, value) in saved {
            a.imm(register, value);
        }
        a.call(entry);
        require(&mut a, fail, Ax, 42);
        // This arithmetic is in the original caller's retained continuation.
        a.group(0, Ax, 100);
        require(&mut a, fail, Ax, 142);
        for (register, value) in saved {
            require(&mut a, fail, register, value);
        }
        a.bin(0x39, Sp, R14);
        a.when(NE, fail);
        require(&mut a, fail, R15, 0);
        for offset in (0..64).step_by(8) {
            a.load(Ax, R14, offset);
            require(&mut a, fail, Ax, 0);
        }
        exit(&mut a, 0);

        a.bind(entry);
        a.push(Bp);
        a.mov(Bp, Sp);
        a.push(Bx);
        a.push(R12);
        a.push(R13);
        a.group_wide(5, Sp, u64::from(old_locals) * 8);
        for index in 0..old_locals {
            a.imm(Ax, 0x4444);
            a.store_offset(Bp, -32 - i64::from(index) * 8, Ax);
        }
        a.imm(Ax, 0xcccc); // The callee operand is not a parameter slot.
        a.push(Ax);
        for value in 1..=arguments {
            a.imm(Ax, u64::from(value));
            a.push(Ax);
        }
        a.mov(Bx, Sp);
        a.address(R8, body);
        a.imm(R9, u64::from(captured + arguments));
        a.imm(R10, u64::from(locals));
        a.address(R12, prefix);
        a.imm(R13, u64::from(captured));
        closures::tail_frame(&mut a, &runtime, arguments, old_locals, true);

        a.bind(body);
        a.mov(Ax, Bp);
        a.group_wide(5, Ax, 24 + u64::from(locals) * 8);
        a.bin(0x39, Sp, Ax);
        a.when(NE, fail);
        for index in 0..captured {
            a.load_offset(Ax, Bp, -32 - i64::from(index) * 8);
            require(&mut a, fail, Ax, 100 + u64::from(index));
        }
        for index in 0..arguments {
            a.load_offset(Ax, Bp, -32 - i64::from(captured + index) * 8);
            require(&mut a, fail, Ax, 1 + u64::from(index));
        }
        for index in captured + arguments..locals {
            a.load_offset(Ax, Bp, -32 - i64::from(index) * 8);
            require(&mut a, fail, Ax, 0);
        }
        for (index, (_, value)) in saved.iter().enumerate() {
            a.load_offset(
                Ax,
                Bp,
                -8 - i64::try_from(index).map_err(|e| e.to_string())? * 8,
            );
            require(&mut a, fail, Ax, *value);
        }
        a.imm(Ax, 42);
        a.lea(Sp, Bp, -24);
        a.pop(R13);
        a.pop(R12);
        a.pop(Bx);
        a.pop(Bp);
        a.ret();

        a.bind(fail);
        exit(&mut a, 70);
        a.align();
        a.bind(prefix);
        for index in 0..captured {
            a.bytes(&(100 + u64::from(index)).to_le_bytes());
        }
        run(a, runtime, "tail-frame-overlap")?;
    }
    Ok(())
}

#[test]
fn native_elf_owner_scan_requires_real_live_buffer_slots() -> std::result::Result<(), String> {
    let mut a = Asm::new();
    let runtime = Runtime::new(&mut a);
    let fail = a.label();
    let entry = a.label();
    let owner = a.label();
    let ordinary = a.label();
    let found = a.label();
    phase::process_context(&mut a);
    a.call(entry);
    exit(&mut a, 0);
    a.bind(entry);
    a.push(Bp);
    a.mov(Bp, Sp);
    a.push(Bx);
    a.push(R12);
    a.push(R13);
    a.group(5, Sp, 32);
    closures::initialize_locals(&mut a, &runtime, 0, 4)
        .map_err(|e| format!("initialize: {e:?}"))?;
    a.address(Ax, ordinary);
    a.store(Bp, -40, Ax);
    closures::owner_range(&mut a, &runtime, 0, 4, fail)
        .map_err(|e| format!("empty range: {e:?}"))?;
    a.address(Ax, owner);
    a.store(Bp, -48, Ax);
    closures::owner_range(&mut a, &runtime, 0, 4, found)
        .map_err(|e| format!("live range: {e:?}"))?;
    a.jump(fail);
    a.bind(found);
    // Clearing a consumed owner is sufficient; a copied ordinary value remains.
    a.imm(Ax, 0);
    a.store(Bp, -48, Ax);
    closures::owner_range(&mut a, &runtime, 0, 4, fail)
        .map_err(|e| format!("consumed range: {e:?}"))?;
    a.lea(Sp, Bp, -24);
    a.pop(R13);
    a.pop(R12);
    a.pop(Bx);
    a.pop(Bp);
    a.ret();
    a.bind(fail);
    exit(&mut a, 70);
    a.align();
    a.bind(owner);
    a.bytes(&8_u64.to_le_bytes());
    a.bytes(&[0; 24]);
    a.bind(ordinary);
    a.bytes(&2_u64.to_le_bytes());
    a.bytes(&17_u64.to_le_bytes());
    run(a, runtime, "tail-owner-slots")
}
