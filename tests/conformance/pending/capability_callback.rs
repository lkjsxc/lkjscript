pub fn main(world: World) -> i64 {
    let callback = move || { world.write_i64(42); 42 };
    callback()
}
