struct Pair { a: Mark, b: Mark }
struct Mark { output: Stdout, value: u8 }
impl Drop for Mark { fn drop(&mut self) { self.output.write(&[self.value]); } }
enum Nested { Pair(Option<Mark>, Mark), Empty }
fn mark(world: &mut World, value: u8) -> Mark { Mark { output: world.stdout(), value } }
pub fn main(mut world: World) {
    let mut out = world.stdout();
    let v = Nested::Pair(Some(mark(&mut world, 65)), mark(&mut world, 66));
    match v { Nested::Pair(Some(m), _) if m.value == 66 => drop(m), Nested::Pair(Some(m), _) => drop(m), _ => () }
    out.write(&[120u8]);
    let mut n = 0u8;
    match (0i64, 0i64) { (0,x) | (x,0) if { n += 1; x == 1 } => (), _ => () }
    out.write(&[b'0' + n]);
    let v2 = Some(Some(mark(&mut world, 67)));
    match &v2 { Some(Some(m)) if m.value == 67 => out.write(&[114u8]), _ => () }
    let numbers = (true, 5i64);
    let x = match numbers { (true, x) | (false, x) => x };
    out.write(&[b'0' + x as u8]);
    let from = Some((7i64, 8i64));
    let Some((a,b)) = from else { return; };
    out.write(&[b'0' + (a+b-10) as u8]);
    let Some(ref saved) = Some(mark(&mut world, 68)) else { return; };
    out.write(&[saved.value + 1]);
    let pair = Pair { a: mark(&mut world, 70), b: mark(&mut world, 71) };
    let Pair { a, .. } = pair;
    drop(a);
    out.write(&[72u8]);
}
