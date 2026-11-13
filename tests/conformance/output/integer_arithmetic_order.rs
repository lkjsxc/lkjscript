fn value(out: &mut Stdout, tag: u8, value: u8) -> u8 { out.write(&[tag]); value }
pub fn main(mut world: World) {
    let mut out = world.stdout();
    let result = value(&mut out, 65, 250).wrapping_add(value(&mut out, 66, 10));
    assert!(result == 4);
    assert!(value(&mut out, 67, 255).checked_add(value(&mut out, 68, 1)).is_none());
    out.write(&[69u8]);
}
