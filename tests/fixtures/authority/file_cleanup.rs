// expect: check
// authority-expect: directory_open; file_close; directory_close
pub fn main(mut tree: Directory) {
    let file = tree.open("language.md").unwrap();
    drop(file);
}
