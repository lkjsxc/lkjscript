fn data_only<F: Data>(mut callback: F) -> i64 where F: FnMut() -> i64 {
    callback()
}
pub fn main(mut world: World) -> i64 {
    let mut output: Stdout = world.stdout();
    let callback = move || {
        output.write(&[52u8, 50u8, 10u8]);
        42i64
    };
    data_only(callback)
}
