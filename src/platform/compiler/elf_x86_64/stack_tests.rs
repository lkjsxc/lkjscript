use super::*;

#[test]
fn native_elf_calls_align_stack_and_restore_every_argument_depth() {
    let mut a = Asm::new();
    let probe = a.label();
    let nested = a.label();
    let fail = a.label();
    for depth in 0..16 {
        for i in 0..depth {
            a.imm(Ax, 100 + i);
            a.push(Ax);
        }
        a.imm(Di, depth);
        a.call(probe);
        a.address(Ax, probe);
        a.indirect(Ax);
        a.address(R11, probe);
        a.indirect(R11);
        for i in (0..depth).rev() {
            a.pop(Ax);
            a.group(7, Ax, 100 + i as i32);
            a.when(NE, fail);
        }
    }
    a.imm(Di, 0);
    a.imm(Ax, 60);
    a.syscall();
    a.bind(probe);
    a.mov(Ax, Sp);
    a.group(4, Ax, 15);
    a.group(7, Ax, 8);
    a.when(NE, fail);
    let no_arguments = a.label();
    a.group(7, Di, 0);
    a.when(E, no_arguments);
    a.load(Cx, Sp, 16); // Original argument base in the aligned call record.
    a.load(Cx, Cx, 0);
    a.lea(Dx, Di, 99);
    a.bin(0x39, Cx, Dx);
    a.when(NE, fail);
    a.bind(no_arguments);
    a.imm(Ax, 777);
    a.push(Ax);
    a.call(nested);
    a.pop(Ax);
    a.group(7, Ax, 777);
    a.when(NE, fail);
    a.ret();
    a.bind(nested);
    a.mov(Ax, Sp);
    a.group(4, Ax, 15);
    a.group(7, Ax, 8);
    a.when(NE, fail);
    a.ret();
    a.bind(fail);
    a.imm(Di, 70);
    a.imm(Ax, 60);
    a.syscall();
    let bytes = elf(&a.finish().unwrap()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("alignment");
    crate::platform::owned_output::publish_executable_create_new(
        &path,
        &bytes,
        Some(128 * 1024),
        "stack alignment test",
    )
    .unwrap();
    let mut child = std::process::Command::new(path)
        .env_clear()
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert_eq!(status.code(), Some(0));
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(5) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("alignment probe timed out");
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

fn stack_probe(bytes: Vec<u8>, constrained: bool) -> std::process::Output {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("stack-probe");
    crate::platform::owned_output::publish_executable_create_new(
        &path,
        &bytes,
        None,
        "native stack probe",
    )
    .unwrap();
    let mut command = if constrained {
        let mut command = std::process::Command::new("/usr/bin/prlimit");
        command.args(["--stack=1048576:1048576", "--core=0", "--"]);
        command.arg(&path);
        command
    } else {
        let mut command = std::process::Command::new("/usr/bin/timeout");
        command.arg("10").arg(&path);
        command
    };
    command.env_clear().output().unwrap()
}

#[test]
fn native_elf_stack_uses_physical_memory_and_respects_external_hard_limits() {
    let mut a = Asm::new();
    super::stack::initialize(&mut a);
    a.group(5, Sp, 16);
    a.imm(Di, 3);
    a.mov(Si, Sp);
    a.imm(Ax, 97);
    a.syscall();
    a.imm(Di, 1);
    a.mov(Si, Sp);
    a.imm(Dx, 16);
    a.imm(Ax, 1);
    a.syscall();
    a.imm(Di, 0);
    a.imm(Ax, 60);
    a.syscall();
    let bytes = elf(&a.finish().unwrap()).unwrap();
    for constrained in [false, true] {
        let output = stack_probe(bytes.clone(), constrained);
        assert!(
            output.status.success(),
            "{:?}: {:?}",
            output.status,
            output.stderr
        );
        assert_eq!(output.stdout.len(), 16);
        let soft = u64::from_le_bytes(output.stdout[..8].try_into().unwrap());
        let hard = u64::from_le_bytes(output.stdout[8..].try_into().unwrap());
        if constrained {
            assert_eq!((soft, hard), (1_048_576, 1_048_576));
        } else {
            assert!(
                soft >= (crate::platform::stack::reservation_bytes().unwrap() as u64).min(hard)
            );
        }
    }
}

#[test]
fn native_elf_non_tail_stack_growth_crosses_inherited_eight_mib() {
    for depth in [2048_u64, 8192, 32768] {
        let mut a = Asm::new();
        let recursive = a.label();
        let unwind = a.label();
        super::stack::initialize(&mut a);
        a.imm(Cx, depth);
        a.call(recursive);
        a.imm(Di, 0);
        a.imm(Ax, 60);
        a.syscall();
        a.bind(recursive);
        a.group(5, Sp, 512);
        a.store(Sp, 0, Cx);
        a.group(7, Cx, 0);
        a.when(E, unwind);
        a.group(5, Cx, 1);
        a.call(recursive);
        a.bind(unwind);
        a.group(0, Sp, 512);
        a.ret();
        let output = stack_probe(elf(&a.finish().unwrap()).unwrap(), false);
        assert!(
            output.status.success(),
            "depth={depth}: {:?} {:?}",
            output.status,
            output.stderr
        );
        eprintln!("native non-tail stack depth={depth} frame_payload_bytes=512");
    }
}
