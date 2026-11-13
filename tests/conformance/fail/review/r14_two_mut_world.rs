struct M { out: Stdout, b: u8 }
impl Drop for M { fn drop(&mut self) { let a: [u8; 1] = [self.b]; self.out.write(a.as_slice()); } }
struct Pair { a: M, b: M }
struct Loud { m: M, tag: u8 }
impl Drop for Loud { fn drop(&mut self) { } }
fn mk(world: &mut World, b: u8) -> M { M { out: world.stdout(), b: b } }
fn eat(m: M) {}
fn eatp(p: Pair) {}
fn peek(m: &M) -> u8 { m.b }
fn c(n: u8) -> bool { n > 3u8 }
fn two(a: &mut World, b: &mut World) {}
pub fn main(mut world: World) { two(&mut world, &mut world); }
