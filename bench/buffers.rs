//! Repeated medium allocation and release, with all storage initialized.
mod common;

#[inline(never)]
fn fresh(length: usize, seed: i64) -> Vec<i64> {
    let mut values = Vec::new();
    let mut index = 0usize;
    while index < length {
        values.push((index as i64) ^ seed);
        index = index + 1usize;
    }
    values
}

pub fn main(mut world: World) {
    let arguments = world.args();
    let count = common::argument(&arguments, 0usize, 200000u64);
    let length = common::argument(&arguments, 1usize, 1000u64) as usize;
    let seed = common::argument(&arguments, 2usize, 7u64) as i64;
    assert!(length != 0usize);
    let mut checksum = 0i64;
    let mut index = 0u64;
    while index < count {
        let values = fresh(length, seed ^ (index as i64));
        checksum = checksum.wrapping_add(values[(index as usize) % length]);
        index = index + 1u64;
    }
    let bytes = common::decimal(checksum as u64);
    let mut out = world.stdout();
    out.write(&bytes);
}
