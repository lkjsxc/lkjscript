use super::*;

fn exit(a: &mut Asm, status: u64) {
    a.imm(Di, status);
    a.imm(Ax, 60);
    a.syscall();
}

fn run_probe(a: Asm) {
    let bytes = elf(&a.finish().unwrap()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("capacity");
    crate::platform::owned_output::publish_executable_create_new(
        &path,
        &bytes,
        Some(128 * 1024),
        "native capacity arithmetic probe",
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
            return;
        }
        if start.elapsed() > std::time::Duration::from_secs(5) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("native capacity arithmetic probe timed out");
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[test]
fn native_byte_extent_has_no_policy_ceiling_and_rejects_negative_lengths() {
    let mut a = Asm::new();
    let fail = a.label();
    for length in [
        0,
        1,
        256 * 1024 * 1024,
        256 * 1024 * 1024 + 1,
        u64::from(u32::MAX),
        u64::from(u32::MAX) + 1,
        i64::MAX as u64,
    ] {
        a.imm(Bx, length);
        bytes::allocation_size(&mut a, fail);
        a.imm(Ax, length + 24);
        a.bin(0x39, Di, Ax);
        a.when(NE, fail);
    }
    for invalid in [i64::MAX as u64 + 1, u64::MAX] {
        let rejected = a.label();
        a.imm(Bx, invalid);
        bytes::allocation_size(&mut a, rejected);
        a.jump(fail);
        a.bind(rejected);
    }
    exit(&mut a, 0);
    a.bind(fail);
    exit(&mut a, 70);
    run_probe(a);
}

#[test]
fn native_buffer_growth_crosses_old_limit_and_preserves_signed_length() {
    let mut a = Asm::new();
    let fail = a.label();
    let maximum = i64::MAX as u64;
    for (length, capacity, expected) in [
        (0, 0, 8),
        (8, 8, 16),
        (256 * 1024 * 1024, 256 * 1024 * 1024, 512 * 1024 * 1024),
        ((1_u64 << 62) - 1, (1_u64 << 62) - 1, maximum - 1),
        (1_u64 << 62, 1_u64 << 62, (1_u64 << 62) + 1),
        (maximum - 1, maximum - 1, maximum),
    ] {
        a.imm(Bx, length);
        a.imm(Ax, capacity);
        buffer::push_length(&mut a, fail);
        buffer::grow_capacity(&mut a);
        a.imm(Cx, expected);
        a.bin(0x39, Ax, Cx);
        a.when(NE, fail);
    }
    for invalid in [maximum, maximum + 1, u64::MAX - 1, u64::MAX] {
        let rejected = a.label();
        a.imm(Bx, invalid);
        buffer::push_length(&mut a, rejected);
        a.jump(fail);
        a.bind(rejected);
    }
    exit(&mut a, 0);
    a.bind(fail);
    exit(&mut a, 70);
    run_probe(a);
}

#[test]
fn native_wide_field_offsets_preserve_flags_without_large_allocations() {
    let mut a = Asm::new();
    let fail = a.label();
    for offset in [
        i64::from(i32::MAX) + 1,
        i64::from(i32::MIN) - 1,
        i64::from(u32::MAX) + 16,
        -i64::from(u32::MAX) - 16,
    ] {
        a.imm(Ax, 41);
        a.push(Ax);
        a.mov(Bx, Sp);
        if offset > 0 {
            a.group_wide(5, Bx, offset as u64);
        } else {
            a.group_wide(0, Bx, offset.unsigned_abs());
        }
        a.group(7, Ax, 41);
        a.load_offset(Cx, Bx, offset);
        a.when(NE, fail);
        a.group(7, Cx, 41);
        a.when(NE, fail);
        a.imm(Ax, 42);
        a.group(7, Ax, 42);
        a.store_offset(Bx, offset, Ax);
        a.when(NE, fail);
        a.pop(Cx);
        a.group(7, Cx, 42);
        a.when(NE, fail);
    }
    exit(&mut a, 0);
    a.bind(fail);
    exit(&mut a, 70);
    run_probe(a);
}

#[test]
fn native_layout_and_absolute_relocation_cross_u32_without_allocating() {
    assert_eq!(
        layout_extent(u32::MAX as usize, 8, 32).unwrap(),
        34_359_738_392
    );
    assert_eq!(
        layout_offset(u32::MAX as usize, 8, 16).unwrap(),
        34_359_738_376
    );
    assert_eq!(
        absolute_address(u32::MAX as usize + 1).unwrap(),
        4_299_165_696
    );
    assert!(layout_extent(usize::MAX, 8, 32).is_err());
    assert!(layout_offset(usize::MAX, 8, 16).is_err());
    assert!(absolute_address(usize::MAX).is_err());
}

#[test]
fn native_code_and_elf_have_no_former_32_mib_ceiling() {
    let mut a = Asm::new();
    let block = vec![0x90; 1024 * 1024];
    for _ in 0..33 {
        a.bytes(&block);
    }
    let code = a.finish().unwrap();
    let image = elf(&code).unwrap();
    assert_eq!(image.len(), TEXT_OFFSET + 33 * 1024 * 1024);
    crate::release_container::inspect_static_elf_bytes(&image).unwrap();
}
