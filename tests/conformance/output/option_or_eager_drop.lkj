// expect: output 527c4c
// witness: {}
struct Tap {
    output: Stdout,
    byte: u8,
}
impl Drop for Tap {
    fn drop(&mut self) {
        self.output.write(&[self.byte]);
    }
}
pub fn main(mut world: World) {
    let left = Some(Tap {
        output: world.stdout(),
        byte: b'L',
    });
    let right = Some(Tap {
        output: world.stdout(),
        byte: b'R',
    });
    let result = left.or(right);
    world.stdout().write(b"|");
    drop(result);
}
