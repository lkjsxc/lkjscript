//! Source runtime over a sealed raw-memory interface. These stubs are ordinary
//! trapping functions in user programs. Only an explicitly prepared runtime
//! bundle substitutes the primitive bodies; names confer no authority.

pub fn raw_load8(_address: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_load64(_address: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_store8(_address: u64, _value: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_store64(_address: u64, _value: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_memory_size() -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_memory_grow(_pages: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_memory_copy(_destination: u64, _source: u64, _bytes: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_heap_tip() -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_set_heap_tip(_value: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_classes() -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_set_classes(_value: u64) -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_fail() -> u64 { panic!("unsealed runtime primitive") }
pub fn raw_drop_step(_frame: u64, _type_id: u64) -> u64 { panic!("unsealed runtime primitive") }

fn add_extent(left: u64, right: u64) -> u64 {
    if left > 18446744073709551615 - right {
        return raw_fail();
    }
    left + right
}

fn ensure_memory(end: u64) -> u64 {
    if end > 18446744073709486080 {
        return raw_fail();
    }
    let required = (end + 65535) >> 16;
    let current = raw_memory_size();
    if required > current {
        let mut capacity = required;
        if current <= 9223372036854775807 {
            let doubled = current * 2;
            if doubled > capacity {
                capacity = doubled;
            }
        }
        if raw_memory_grow(capacity - current) == 18446744073709551615 {
            if raw_memory_grow(required - current) == 18446744073709551615 {
                return raw_fail();
            }
        }
    }
    0
}

// Size classes describe the payload, so a power-of-two Vec capacity fills its
// class. Each block has a {capacity, next_free} header followed by at least 16
// payload bytes. Headers add 16 bytes; payloads remain eight-byte aligned.
// Freed blocks are reused in constant list work. Tip growth keeps its address.
fn classes() -> u64 {
    let old = raw_classes();
    if old != 0 {
        return old;
    }
    let base = raw_heap_tip();
    let end = add_extent(base, 512);
    ensure_memory(end);
    let mut index: u64 = 0;
    while index < 64 {
        raw_store64(base + index * 8, 0);
        index = index + 1;
    }
    raw_set_heap_tip(end);
    raw_set_classes(base);
    base
}

fn payload_capacity(bytes: u64) -> u64 {
    let mut size: u64 = 16;
    while size < bytes {
        if size > 9223372036854775807 {
            return raw_fail();
        }
        size = size * 2;
    }
    size
}

fn class_index(size: u64) -> u64 {
    let mut extent: u64 = 16;
    let mut class: u64 = 0;
    while extent < size {
        extent = extent * 2;
        class = class + 1;
    }
    class
}

pub fn allocate(bytes: u64) -> u64 {
    let size = payload_capacity(bytes);
    let head = classes() + class_index(size) * 8;
    let old = raw_load64(head);
    if old != 0 {
        raw_store64(head, raw_load64(old + 8));
        raw_store64(old + 8, 0);
        return old + 16;
    }
    let block = raw_heap_tip();
    let pointer = add_extent(block, 16);
    let end = add_extent(pointer, size);
    ensure_memory(end);
    raw_set_heap_tip(end);
    raw_store64(block, size);
    raw_store64(block + 8, 0);
    pointer
}

// Reallocation transfers the block; callers retain no pointer into its payload.
// Only the tip can grow in place. Other blocks move, copy their entire old
// capacity, and release the old class after the copy succeeds.
pub fn reallocate(pointer: u64, bytes: u64) -> u64 {
    if pointer == 0 {
        return allocate(bytes);
    }
    let old_size = raw_load64(pointer - 16);
    if bytes <= old_size {
        return pointer;
    }
    if add_extent(pointer, old_size) == raw_heap_tip() {
        let size = payload_capacity(bytes);
        let end = add_extent(pointer, size);
        ensure_memory(end);
        raw_store64(pointer - 16, size);
        raw_set_heap_tip(end);
        return pointer;
    }
    let replacement = allocate(bytes);
    raw_memory_copy(replacement, pointer, old_size);
    release(pointer);
    replacement
}

pub fn release(pointer: u64) -> u64 {
    if pointer == 0 {
        return 0;
    }
    let block = pointer - 16;
    let size = raw_load64(block);
    let head = raw_classes() + class_index(size) * 8;
    raw_store64(block + 8, raw_load64(head));
    raw_store64(head, block);
    0
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

// Each cleanup frame is {value, type, next_child, child_type}. Glue yields one
// child at a time. The source walker owns the growing depth-first stack, so even
// destruction of a deep owner chain does not recurse on the native call stack.
pub fn drop_walk(mut base: u64) -> u64 {
    let mut top: u64 = 0;
    let mut capacity: u64 = 32;
    loop {
        let frame = base + top;
        let child = raw_drop_step(frame, raw_load64(frame + 8));
        if child == 0 {
            if top == 0 {
                return release(base);
            }
            top = top - 32;
        } else {
            let child_type = raw_load64(frame + 24);
            top = add_extent(top, 32);
            if top == capacity {
                if capacity > 9223372036854775807 {
                    return raw_fail();
                }
                let larger = capacity * 2;
                base = reallocate(base, larger);
                capacity = larger;
            }
            let next = base + top;
            raw_store64(next, child);
            raw_store64(next + 8, child_type);
            raw_store64(next + 16, 0);
            raw_store64(next + 24, 0);
        }
    }
}
