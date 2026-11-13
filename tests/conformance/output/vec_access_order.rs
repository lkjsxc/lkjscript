struct M {
    out: Stdout,
    b: u8,
}
impl Drop for M {
    fn drop(&mut self) {
        self.out.write(&[self.b]);
    }
}
struct P {
    a: M,
    b: M,
}
fn tick(world: &mut World, byte: u8) {
    let mut out = world.stdout();
    out.write(&[byte]);
}
fn make(world: &mut World, byte: u8) -> M {
    tick(world, byte + 100);
    M {
        out: world.stdout(),
        b: byte,
    }
}
fn receiver<'a, T>(world: &mut World, values: &'a Vec<T>) -> &'a Vec<T> {
    tick(world, 88u8);
    values
}
fn chosen_index(world: &mut World, index: usize) -> usize {
    tick(world, 89u8);
    index
}

pub fn main(mut world: World) {
    let mut values: Vec<M> = Vec::new();
    values.push(make(&mut world, 4u8));
    values.push(make(&mut world, 5u8));
    {
        let item = receiver(&mut world, &values).get(chosen_index(&mut world, 1usize));
        let copied = item;
        drop(item);
        match copied {
            Some(value) => tick(&mut world, value.b),
            None => tick(&mut world, 65u8),
        }
    }
    drop(values);
    tick(&mut world, 99u8);
}
