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
struct Q {
    mark: M,
    value: i64,
}
impl PartialEq for Q {
    fn eq(&self, other: &Self) -> bool {
        assert!(other.value == 42);
        assert!(self.value != -1);
        self.value == other.value
    }
    fn ne(&self, _other: &Self) -> bool {
        false
    }
}
fn query(world: &mut World) -> Q {
    Q {
        mark: make(world, 11u8),
        value: 42,
    }
}

pub fn main(mut world: World) {
    let mut values: Vec<Q> = Vec::new();
    values.push(Q {
        mark: make(&mut world, 1u8),
        value: 1i64,
    });
    values.push(Q {
        mark: make(&mut world, 2u8),
        value: 42i64,
    });
    values.push(Q {
        mark: make(&mut world, 3u8),
        value: -1i64,
    });
    let answer = receiver(&mut world, &values).contains(&query(&mut world));
    tick(&mut world, if answer { 65u8 } else { 66u8 });
    drop(values);
    tick(&mut world, 99u8);
}
