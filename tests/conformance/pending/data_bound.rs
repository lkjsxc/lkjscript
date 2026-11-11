fn data_only<F: Data>(callback: F) -> i64 where F: Fn() -> i64 { callback() }
pub fn main(world: World) -> i64 {
    let callback = move || { world.write_i64(42); 42 };
    data_only(callback)
}
