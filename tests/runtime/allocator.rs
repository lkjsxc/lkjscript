// Appended to the explicit runtime bundle by the model harness.
pub fn probe_allocator() -> u64 {
    let a = allocate(8);
    let b = allocate(32);
    let c = allocate(512);
    if raw_load64(a - 16) != 16 || raw_load64(b - 16) != 32 || raw_load64(c - 16) != 512 { return 6; }
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
    // A fresh large payload sits at the tip. Power-of-two capacities do not
    // pay another size class for the header, and growth retains the address.
    let tip = allocate(2097152);
    if raw_load64(tip - 16) != 2097152 || raw_heap_tip() != tip + 2097152 { return 7; }
    raw_store64(tip, 12345);
    raw_store8(tip + 2097151, 201);
    let grown = reallocate(tip, 4194304);
    if grown != tip || raw_heap_tip() != tip + 4194304 { return 8; }
    if raw_load64(grown) != 12345 || raw_load8(grown + 2097151) != 201 { return 9; }
    if reallocate(grown, 17) != grown || raw_load64(grown - 16) != 4194304 { return 10; }
    release(grown);
    if allocate(4194304) != grown { return 11; }
    release(grown);

    // A live neighboring block prevents tip growth. The moved block preserves
    // all old payload bytes and the neighbor, then returns its old class.
    let left = allocate(32);
    let guard = allocate(64);
    raw_store64(left, 6789);
    raw_store64(left + 24, 9876);
    raw_store64(guard, 54321);
    let moved = reallocate(left, 129);
    if moved == left || raw_load64(moved - 16) != 256 { return 12; }
    if raw_load64(moved) != 6789 || raw_load64(moved + 24) != 9876 || raw_load64(guard) != 54321 { return 13; }
    if allocate(32) != left { return 14; }
    release(left);
    release(guard);
    release(moved);
    let empty = reallocate(0, 7);
    if raw_load64(empty - 16) != 16 { return 15; }
    release(empty);
    42
}
pub fn probe_overflow() -> u64 { allocate(18446744073709551615) }
