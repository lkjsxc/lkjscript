fn work(mut value: u64) -> u64 {
    let mut index = 0u64;
    while index < 160000000u64 {
        value = value.wrapping_mul(6364136223846793005u64) ^ index;
        index += 1;
    }
    value
}
fn recurse(depth: u64) -> u64 {
    if depth == 0 { work(17) } else { recurse(depth - 1) }
}
fn main(mut world: World) {
    let result = recurse(96);
    world.stdout().write(format!("{result}\n").as_bytes());
}
