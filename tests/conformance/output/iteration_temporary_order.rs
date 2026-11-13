struct M {
    out: Stdout,
    b: u8,
}
impl Drop for M {
    fn drop(&mut self) {
        self.out.write(&[self.b]);
    }
}
fn tick(world: &mut World, b: u8) {
    let mut out = world.stdout();
    out.write(&[b]);
}
fn make(world: &mut World, b: u8) -> M {
    tick(world, b + 100);
    M {
        out: world.stdout(),
        b,
    }
}
fn made(world: &mut World) -> Vec<M> {
    tick(world, 80);
    vec![make(world, 1), make(world, 2)]
}
fn step(world: &mut World, mode: u8) {
    for item in &made(world) {
        let temporary = make(world, 30 + mode);
        tick(world, item.b + 10);
        if mode == 0 {
            continue;
        }
        if mode == 1 {
            break;
        }
        if mode == 2 {
            return;
        }
        drop(temporary);
    }
    tick(world, 60 + mode);
}
pub fn main(mut world: World) {
    step(&mut world, 0);
    step(&mut world, 1);
    step(&mut world, 2);
    step(&mut world, 3);
    tick(&mut world, 90);
}
