//! A serial multiply/xor recurrence prevents algebraic loop removal.
mod common;

pub fn main(mut world: World) {
    let arguments = world.args();
    let count = common::argument(&arguments, 0usize, 200000000u64);
    let mut value = common::argument(&arguments, 1usize, 7u64);
    let mut index = 0u64;
    while index < count {
        value = value.wrapping_mul(6364136223846793005u64).wrapping_add(1u64);
        value = value ^ (value >> 23u64);
        index = index + 1u64;
    }
    let bytes = common::decimal(value);
    let mut out = world.stdout();
    out.write(&bytes);
}
