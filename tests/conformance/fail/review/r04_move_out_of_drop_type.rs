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
pub fn main(mut world: World) { let d: Loud = Loud { m: mk(&mut world, 1u8), tag: 2u8 }; let m: M = d.m; }
