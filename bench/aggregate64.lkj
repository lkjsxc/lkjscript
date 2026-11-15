//! A 64-byte value crosses three ordinary call boundaries in both directions.
mod common;

#[derive(Clone, Copy)]
struct Record {
    a: u64,
    b: u64,
    c: u64,
    d: u64,
    e: u64,
    f: u64,
    g: u64,
    h: u64,
}

#[inline(never)]
fn third(mut value: Record, index: u64) -> Record {
    value.a = value.a.wrapping_add(index) ^ value.h;
    value.d = value.d.wrapping_mul(1664525u64).wrapping_add(value.c);
    value.g = value.g.wrapping_add(value.b);
    value
}

#[inline(never)]
fn second(mut value: Record, index: u64) -> Record {
    value.b = value.b.wrapping_add(index ^ value.e);
    value.f = value.f.wrapping_mul(1013904223u64).wrapping_add(value.h);
    third(value, index)
}

#[inline(never)]
fn first(mut value: Record, index: u64) -> Record {
    value.c = value.c.wrapping_add(index ^ value.f);
    value.e = value.e.wrapping_add(value.g);
    value.h = value.h.wrapping_mul(1103515245u64).wrapping_add(12345u64);
    second(value, index)
}

pub fn main(mut world: World) {
    let arguments = world.args();
    let count = common::argument(&arguments, 0usize, 10000000u64);
    let seed = common::argument(&arguments, 1usize, 7u64);
    let mut value = Record {
        a: seed,
        b: seed ^ 1u64,
        c: seed ^ 2u64,
        d: seed ^ 3u64,
        e: seed ^ 4u64,
        f: seed ^ 5u64,
        g: seed ^ 6u64,
        h: seed ^ 7u64,
    };
    let mut index = 0u64;
    while index < count {
        value = first(value, index);
        index += 1u64;
    }
    let bytes = common::decimal(
        value.a ^ value.b ^ value.c ^ value.d ^ value.e ^ value.f ^ value.g ^ value.h,
    );
    let mut output = world.stdout();
    output.write(&bytes);
}
