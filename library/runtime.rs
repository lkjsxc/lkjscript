//! Source allocator over sealed raw addresses. Ordinary application stubs trap.
//! Allocator metadata is private to this runtime; programs receive payload pointers.

pub fn raw_load8(_address: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_load64(_address: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_store8(_address: u64, _value: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_store64(_address: u64, _value: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_memory_copy(_destination: u64, _source: u64, _bytes: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_map(_bytes: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_unmap(_address: u64, _bytes: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_remap(_address: u64, _old_bytes: u64, _new_bytes: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_allocator_state() -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_fail() -> u64 { panic!("unsealed runtime primitive") }

fn add_extent(left: u64, right: u64) -> u64 {
    if left > 18446744073709551615 - right {
        return raw_fail();
    }
    left + right
}

fn payload_class(bytes: u64) -> u64 {
    let mut size: u64 = 16;
    let mut class: u64 = 0;
    while size < bytes {
        if size > 9223372036854775807 {
            return raw_fail();
        }
        size = size * 2;
        class = class + 1;
    }
    class
}

// Free-list heads occupy the first 64 words of allocator state. The next three
// words are the current region's cursor, remaining bytes and next region size.
// Regions grow geometrically; a refused request is retried down to the pages
// needed by this allocation. Retained regions supply slabs and medium blocks.
fn take_region(bytes: u64) -> u64 {
    let state = raw_allocator_state();
    let mut cursor = raw_load64(state + 512);
    let mut remaining = raw_load64(state + 520);
    if remaining < bytes {
        let minimum = add_extent(bytes, 4095) & !4095u64;
        let mut size = raw_load64(state + 528);
        if size == 0 {
            size = 65536;
        }
        if size < minimum {
            size = minimum;
        }
        cursor = raw_map(size);
        while cursor == 0 && size > minimum {
            size = (size / 2) & !4095u64;
            if size < minimum {
                size = minimum;
            }
            cursor = raw_map(size);
        }
        if cursor == 0 {
            return raw_fail();
        }
        remaining = size;
        let mut next = size;
        if next <= 9223372036854775807 {
            next = next * 2;
        }
        raw_store64(state + 528, next);
    }
    raw_store64(state + 512, cursor + bytes);
    raw_store64(state + 520, remaining - bytes);
    cursor
}

// Each block has {class, next_free} followed by a power-of-two payload.
// Payloads through 1 MiB are reused by class. Small blocks are carved from
// 4096-byte slabs inside regions; medium blocks are carved directly. Larger
// blocks have independent mappings so growth can remap without copying.
pub fn allocate(bytes: u64) -> u64 {
    let class = payload_class(bytes);
    let size = 16u64 << class;
    let block_size = add_extent(size, 16);
    if size > 1048576 {
        let block = raw_map(block_size);
        if block == 0 {
            return raw_fail();
        }
        raw_store64(block, class);
        raw_store64(block + 8, 0);
        return block + 16;
    }
    let head = raw_allocator_state() + class * 8;
    let old = raw_load64(head);
    if old != 0 {
        raw_store64(head, raw_load64(old + 8));
        raw_store64(old + 8, 0);
        return old + 16;
    }
    if block_size > 4096 {
        let block = take_region(block_size);
        raw_store64(block, class);
        raw_store64(block + 8, 0);
        return block + 16;
    }
    let slab = take_region(4096);
    let count = 4096 / block_size;
    let mut index: u64 = count;
    let mut next: u64 = 0;
    while index != 0 {
        index = index - 1;
        let block = slab + index * block_size;
        raw_store64(block, class);
        raw_store64(block + 8, next);
        next = block;
    }
    raw_store64(head, raw_load64(slab + 8));
    raw_store64(slab + 8, 0);
    slab + 16
}

pub fn release(pointer: u64) -> u64 {
    if pointer == 0 {
        return 0;
    }
    let block = pointer - 16;
    let class = raw_load64(block);
    let size = 16u64 << class;
    if size > 1048576 {
        if raw_unmap(block, add_extent(size, 16)) != 0 {
            return raw_fail();
        }
        return 0;
    }
    let head = raw_allocator_state() + class * 8;
    raw_store64(block + 8, raw_load64(head));
    raw_store64(head, block);
    0
}

// No caller retains a pointer into a block whose ownership is transferred here.
// Shrinking keeps capacity. Dedicated mappings grow with mremap; only pooled
// blocks copy their payload when moving into a larger size class.
pub fn reallocate(pointer: u64, bytes: u64) -> u64 {
    if pointer == 0 {
        return allocate(bytes);
    }
    let block = pointer - 16;
    let old_size = 16u64 << raw_load64(block);
    if bytes <= old_size {
        return pointer;
    }
    if old_size > 1048576 {
        let class = payload_class(bytes);
        let size = 16u64 << class;
        let replacement = raw_remap(block, add_extent(old_size, 16), add_extent(size, 16));
        if replacement == 0 {
            return raw_fail();
        }
        raw_store64(replacement, class);
        return replacement + 16;
    }
    let replacement = allocate(bytes);
    raw_memory_copy(replacement, pointer, old_size);
    release(pointer);
    replacement
}

pub fn valid_utf8_bytes(data: u64, length: u64) -> u64 {
    let mut index: u64 = 0;
    let mut remaining: u64 = 0;
    let mut low: u64 = 128;
    let mut high: u64 = 191;
    while index < length {
        let byte = raw_load8(data + index);
        if remaining != 0 {
            if byte < low || byte > high {
                return 0;
            }
            remaining = remaining - 1;
            low = 128;
            high = 191;
        } else if byte >= 128 {
            low = 128;
            high = 191;
            if byte >= 194 && byte <= 223 {
                remaining = 1;
            } else if byte >= 224 && byte <= 239 {
                remaining = 2;
                if byte == 224 {
                    low = 160;
                }
                if byte == 237 {
                    high = 159;
                }
            } else if byte >= 240 && byte <= 244 {
                remaining = 3;
                if byte == 240 {
                    low = 144;
                }
                if byte == 244 {
                    high = 143;
                }
            } else {
                return 0;
            }
        }
        index = index + 1;
    }
    if remaining == 0 { 1 } else { 0 }
}

pub fn utf8_validate(data: u64, length: u64) -> u64 {
    valid_utf8_bytes(data, length)
}
