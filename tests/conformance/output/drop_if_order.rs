struct Traced { output: Stdout, byte: u8 }
impl Drop for Traced {
    fn drop(&mut self) {
        let bytes = [self.byte];
        self.output.write(&bytes);
    }
}
struct Pair { first: Traced, second: Traced }
pub fn main(mut world: World) {
    let mut pair = Pair {
        first: Traced { output: world.stdout(), byte: 65 },
        second: Traced { output: world.stdout(), byte: 67 },
    };
    let moved = pair.first;
    pair.first = Traced { output: world.stdout(), byte: 66 };
    pair.second = Traced { output: world.stdout(), byte: 68 };
    drop(moved);
}
