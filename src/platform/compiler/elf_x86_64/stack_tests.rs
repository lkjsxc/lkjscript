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
        128 * 1024,
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
