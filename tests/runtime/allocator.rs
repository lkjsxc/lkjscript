// Appended to the explicit runtime bundle by the model harness.
pub fn probe_allocator() -> u64 {
    let a = allocate(8);
    let b = allocate(32);
    let c = allocate(512);
    raw_store64(a, 11);
    raw_store64(b, 22);
    raw_store64(c, 33);
    release(a);
    release(b);
    let d = allocate(8);
    let e = allocate(32);
    if d != a || e != b || raw_load64(c) != 33 { return 1; }
    raw_store64(d, 44);
    raw_store64(e, 55);
    if raw_load64(c) != 33 { return 2; }
    release(c);
    release(d);
    release(e);
    let mut size: u64 = 1;
    while size <= 1048576 {
        let block = allocate(size);
        raw_store8(block + size - 1, 173);
        if raw_load8(block + size - 1) != 173 { return 3; }
        release(block);
        size = size * 2;
    }
    let peak = raw_heap_tip();
    size = 1;
    while size <= 1048576 {
        let block = allocate(size);
        release(block);
        size = size * 2;
    }
    if raw_heap_tip() != peak { return 4; }
    let block = allocate(32);
    let mut index: u64 = 0;
    while index < 16 {
        raw_store8(block + index, index);
        index = index + 1;
    }
    raw_memory_copy(block + 4, block, 12);
    if raw_load8(block + 4) != 0 || raw_load8(block + 15) != 11 { return 5; }
    release(block);
    42
}
pub fn probe_overflow() -> u64 { allocate(18446744073709551615) }
