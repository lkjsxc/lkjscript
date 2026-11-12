struct Mark { out: Stdout, byte: u8 }
impl Drop for Mark {
    fn drop(&mut self) { self.out.write([self.byte].as_slice()); }
}
struct Pair { left: Mark, right: Mark }
fn mark(world: &mut World, byte: u8) -> Mark { Mark { out: world.stdout(), byte: byte } }
fn tick(world: &mut World, byte: u8) { let mut out: Stdout = world.stdout(); out.write([byte].as_slice()); }
pub fn main(mut world: World) {
    {
        let a: Mark = mark(&mut world, 65u8);
        if true { drop(a); }
        tick(&mut world, 120u8);
        let b: Mark = mark(&mut world, 66u8);
        if false { drop(b); }
        tick(&mut world, 121u8);
    }
    {
        let mut p: Pair = Pair { left: mark(&mut world, 67u8), right: mark(&mut world, 68u8) };
        if true { drop(p.left); }
        if false { drop(p.right); }
        tick(&mut world, 122u8);
        p.left = mark(&mut world, 69u8);
        tick(&mut world, 119u8);
    }
    {
        let mut q: Mark = mark(&mut world, 70u8);
        if true { drop(q); }
        q = mark(&mut world, 71u8);
        tick(&mut world, 118u8);
    }
    {
        let u: Mark;
        if true { u = mark(&mut world, 72u8); } else { u = mark(&mut world, 73u8); }
        drop(u);
        let absent: Mark;
        if false { absent = mark(&mut world, 74u8); }
        let present: Mark;
        if true { present = mark(&mut world, 75u8); }
        tick(&mut world, 117u8);
    }
}
