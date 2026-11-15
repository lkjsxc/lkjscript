//! Add/xor/mask recurrence with runtime inputs and no library work in the loop.
mod common;

pub fn main(mut world: World) {
    let arguments = world.args();
    let count = common::argument(&arguments, 0usize, 200000000u64);
    let mut value = common::argument(&arguments, 1usize, 7u64) & 2147483647u64;
    let mut index = 0u64;
    while index < count {
        value = ((value + (index & 255u64)) ^ 23130u64) & 2147483647u64;
        index = index + 1u64;
    }
    let bytes = common::decimal(value);
    let mut out = world.stdout();
    out.write(&bytes);
}
