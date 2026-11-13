struct Stamp<'a> {
    output: &'a mut Stdout,
}
impl Drop for Stamp<'_> {
    fn drop(&mut self) {
        self.output.write(&[97u8]);
    }
}
fn use_output<'a>(output: &'a mut Stdout) {
    let stamp: Stamp<'a> = Stamp {
        output: &mut *output,
    };
    stamp.output.write(&[49u8]);
    drop(stamp);
}
pub fn main(mut world: World) {
    let mut output = world.stdout();
    output.write(&[65u8]);
    use_output(&mut output);
}
