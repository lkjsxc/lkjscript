//! Source allocator over sealed raw addresses. Ordinary application stubs trap.
//! Allocator metadata is private to this runtime; programs receive payload pointers.

pub fn raw_load8(_address: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_load64(_address: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_store8(_address: u64, _value: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_store64(_address: u64, _value: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_memory_copy(_destination: u64, _source: u64, _bytes: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_map(_bytes: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_unmap(_address: u64, _bytes: u64) -> u64 { panic!("unsealed runtime primitive") }
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

// Each block has {class, next_free} followed by a power-of-two payload.
// Small blocks share an anonymous Linux page; the free list retains reusable
// blocks. Large blocks have independent mappings and return them on release.
// A slab occupies one x86-64 Linux base page (4096 bytes).
pub fn allocate(bytes: u64) -> u64 {
    let class = payload_class(bytes);
    let size = 16u64 << class;
    let block_size = add_extent(size, 16);
    if block_size > 4096 {
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
    let slab = raw_map(4096);
    if slab == 0 {
        return raw_fail();
    }
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
    let block_size = add_extent(16u64 << class, 16);
    if block_size > 4096 {
        if raw_unmap(block, block_size) != 0 {
            return raw_fail();
        }
        return 0;
    }
    let head = raw_allocator_state() + class * 8;
    raw_store64(block + 8, raw_load64(head));
    raw_store64(head, block);
    0
}

// Reallocation transfers the block; no caller retains a pointer into its payload.
// Shrinking retains capacity. Growth preserves bytes before releasing the old block.
pub fn reallocate(pointer: u64, bytes: u64) -> u64 {
    if pointer == 0 {
        return allocate(bytes);
    }
    let old_size = 16u64 << raw_load64(pointer - 16);
    if bytes <= old_size {
        return pointer;
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
