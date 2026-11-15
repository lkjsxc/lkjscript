// expect: reject E_BORROW
// witness: {"rustAccept":false,"rustCode":"E0596","world":true}
fn probe(ready: &Readiness, file: &mut File) {
    let _status = ready.watch_file(file, 1usize, 1u8);
}
