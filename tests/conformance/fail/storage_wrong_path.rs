// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0308","world":true}
fn probe(directory: &mut Directory) {
    let _result = directory.create_new(b"record");
}
