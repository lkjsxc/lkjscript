pub fn main(mut root: Directory) {
    let mut file = root.create_new("record").unwrap();
    file.lock_exclusive().unwrap();
    file.write_all(b"record").unwrap();
    file.sync().unwrap();
    drop(file);
    let mut append = root.open_append("record").unwrap();
    append.truncate(6usize).unwrap();
    append.sync().unwrap();
    root.sync().unwrap();
}
