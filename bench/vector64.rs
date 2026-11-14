//! Build and walk inline 64-byte records, observing every field.
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

pub fn main(mut world: World) {
    let arguments = world.args();
    let count = common::argument(&arguments, 0usize, 10000000u64);
    let length = common::argument(&arguments, 1usize, 1000000u64) as usize;
    let seed = common::argument(&arguments, 2usize, 7u64);
    assert!(length != 0usize);
    let mut values = Vec::new();
    let mut index = 0usize;
    while index < length {
        let word = (index as u64) ^ seed;
        values.push(Record {
            a: word,
            b: word.wrapping_mul(3u64),
            c: word.wrapping_mul(5u64),
            d: word.wrapping_mul(7u64),
            e: word.wrapping_mul(11u64),
            f: word.wrapping_mul(13u64),
            g: word.wrapping_mul(17u64),
            h: word.wrapping_mul(19u64),
        });
        index += 1usize;
    }
    let mut checksum = 0u64;
    let mut read = 0u64;
    index = 0usize;
    while read < count {
        let value = values[index];
        checksum = checksum.wrapping_add(
            value.a ^ value.b ^ value.c ^ value.d ^ value.e ^ value.f ^ value.g ^ value.h,
        );
        index += 1usize;
        if index == length {
            index = 0usize;
        }
        read += 1u64;
    }
    let bytes = common::decimal(checksum);
    let mut output = world.stdout();
    output.write(&bytes);
}
