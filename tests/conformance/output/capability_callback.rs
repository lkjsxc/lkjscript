pub fn main(mut world: World) {
    let mut output: Stdout = world.stdout();
    let mut callback = move || {
        output.write(&[52u8, 50u8, 10u8]);
        42i64
    };
    assert_eq!(callback(), 42);
}
