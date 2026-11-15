//! Build inline i64 storage, then cycle through it for a runtime read count.
mod common;

pub fn main(mut world: World) {
    let arguments = world.args();
    let count = common::argument(&arguments, 0usize, 100000000u64);
    let length = common::argument(&arguments, 1usize, 5000000u64) as usize;
    let seed = common::argument(&arguments, 2usize, 7u64) as i64;
    assert!(length != 0usize);
    let mut values = Vec::new();
    let mut index = 0usize;
    while index < length {
        values.push((index as i64) ^ seed);
        index = index + 1usize;
    }
    let mut checksum = 0i64;
    let mut read = 0u64;
    index = 0usize;
    while read < count {
        checksum = checksum.wrapping_add(values[index]);
        index = index + 1usize;
        if index == length {
            index = 0usize;
        }
        read = read + 1u64;
    }
    let bytes = common::decimal(checksum as u64);
    let mut out = world.stdout();
    out.write(&bytes);
}
