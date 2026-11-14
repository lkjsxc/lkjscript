//! Runtime count and seed keep the aggregate recurrence observable.
mod common;

#[derive(Clone, Copy)]
struct Pair {
    left: u64,
    right: u64,
}

#[inline(never)]
fn step(pair: Pair, index: u64) -> Pair {
    Pair {
        left: pair.left.wrapping_add(index) ^ pair.right,
        right: pair.right.wrapping_mul(1664525u64).wrapping_add(1013904223u64),
    }
}

pub fn main(mut world: World) {
    let arguments = world.args();
    let count = common::argument(&arguments, 0usize, 20000000u64);
    let seed = common::argument(&arguments, 1usize, 7u64);
    let mut pair = Pair { left: seed, right: seed + 1u64 };
    let mut index = 0u64;
    while index < count {
        pair = step(pair, index);
        index = index + 1u64;
    }
    let bytes = common::decimal(pair.left ^ pair.right);
    let mut out = world.stdout();
    out.write(&bytes);
}
