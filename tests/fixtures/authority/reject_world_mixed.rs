// expect: check
// authority-reject: E_ENTRY; World must be the entry's only parameter
pub fn main(world: World, output: Stdout) {
    drop(world);
    drop(output);
}
