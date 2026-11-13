struct Mark {
    out: Stdout,
    byte: u8,
}
impl Drop for Mark {
    fn drop(&mut self) {
        self.out.write(&[self.byte]);
    }
}
pub fn main(mut world: World) {
    let mut output = world.stdout();
    let mut values = vec![
        Mark {
            out: world.stdout(),
            byte: 1u8,
        },
        Mark {
            out: world.stdout(),
            byte: 2u8,
        },
        Mark {
            out: world.stdout(),
            byte: 3u8,
        },
        Mark {
            out: world.stdout(),
            byte: 4u8,
        },
    ];
    values.retain(|value| {
        output.write(&[value.byte + 10u8]);
        value.byte % 2u8 == 1u8
    });
    drop(values);
    output.write(&[99u8]);
}
