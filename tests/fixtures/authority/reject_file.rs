// expect: check
// authority-reject: E_ENTRY; accept Directory and open the file inside the entry
pub fn main(file: File) {
    drop(file);
}
