// The launcher controls umask; no chmod may precede inspection of "atomic".
pub fn main(mut world: World) {
    let arguments = world.args();
    let root = String::from_utf8(arguments[0].clone()).unwrap();
    let mut directory = world.directory(&root).unwrap();
    let mut atomic = directory.create_private("atomic").unwrap();
    atomic.write_all(b"private").unwrap();
    atomic.sync().unwrap();
    drop(atomic);
    match directory.create_private("atomic") { Err(17) => {}, _ => panic!("exclusive private creation") }
    let mut repaired = directory.create_private("repaired").unwrap();
    repaired.set_private().unwrap();
    repaired.write_all(b"owner access").unwrap();
    repaired.sync().unwrap();
    directory.sync().unwrap();
    world.stdout().write(b"private files ok\n");
}
