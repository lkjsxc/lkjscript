fn make(out: &mut Stdout, tag: u8) -> Vec<i64> {
    out.write(&[tag]);
    vec![19i64, 23]
}
fn first<T: Copy>(values: &[T]) -> T { values[0] }
fn join<T: Copy>(left: &[T], right: &[T]) -> (T, T) { (left[0], right[1]) }
pub fn main(mut world: World) {
    let mut out = world.stdout();
    assert!(first(&make(&mut out, 65)) == 19);
    let (left, right) = join(&make(&mut out, 66), &make(&mut out, 67));
    assert!(left + right == 42);
    out.write(&[68u8]);
}
