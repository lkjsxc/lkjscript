// Runtime probe: expected u64 result 42 when appended to the sealed runtime source.
pub fn probe_allocator() -> u64 {
    if payload_class(0) != 0 || payload_class(16) != 0 || payload_class(17) != 1 || payload_class(9223372036854775808) != 59 { return 1; }
    let a = allocate(8);
    let b = allocate(32);
    raw_store64(a, 11);
    raw_store64(b, 22);
    release(a);
    let reused = allocate(8);
    if reused != a || raw_load64(b) != 22 { return 2; }
    release(reused);
    release(b);

    // Multiple simultaneously live slabs must remain disjoint.
    let slots = allocate(2400);
    let mut index: u64 = 0;
    while index < 300 {
        let block = allocate(16);
        raw_store64(slots + index * 8, block);
        raw_store64(block, index);
        raw_store64(block + 8, index + 1000);
        index = index + 1;
    }
    index = 0;
    while index < 300 {
        let block = raw_load64(slots + index * 8);
        if raw_load64(block) != index || raw_load64(block + 8) != index + 1000 { return 3; }
        release(block);
        index = index + 1;
    }
    release(slots);

    let mut size: u64 = 1;
    while size <= 1048576 {
        let block = allocate(size);
        raw_store8(block + size - 1, 173);
        if raw_load8(block + size - 1) != 173 { return 4; }
        release(block);
        size = size * 2;
    }

    let block = allocate(32);
    index = 0;
    while index < 16 {
        raw_store8(block + index, index);
        index = index + 1;
    }
    raw_memory_copy(block + 4, block, 12);
    if raw_load8(block + 4) != 0 || raw_load8(block + 15) != 11 { return 5; }
    release(block);

    let left = allocate(32);
    let guard = allocate(64);
    raw_store64(left, 6789);
    raw_store64(left + 24, 9876);
    raw_store64(guard, 54321);
    let grown = reallocate(left, 129);
    if grown == left || raw_load64(grown - 16) != 4 { return 6; }
    if raw_load64(grown) != 6789 || raw_load64(grown + 24) != 9876 || raw_load64(guard) != 54321 { return 7; }
    if reallocate(grown, 17) != grown { return 8; }
    if allocate(32) != left { return 9; }
    release(left);
    release(guard);
    release(grown);

    let large = allocate(8192);
    raw_store64(large, 12345);
    raw_store8(large + 8191, 201);
    let larger = reallocate(large, 16384);
    if raw_load64(larger) != 12345 || raw_load8(larger + 8191) != 201 { return 10; }
    release(larger);

    let empty = reallocate(0, 7);
    raw_store64(empty, 42);
    let answer = raw_load64(empty);
    release(empty);
    release(0);
    answer
}
pub fn probe_overflow() -> u64 { allocate(18446744073709551615) }
