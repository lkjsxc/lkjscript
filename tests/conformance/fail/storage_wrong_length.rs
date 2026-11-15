// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0308","world":true}
fn probe(file: &mut File) {
    let _result = file.truncate(-1i64);
}
