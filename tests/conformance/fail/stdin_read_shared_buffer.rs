// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0308","world":true}
fn probe(mut input: Stdin) {
    let bytes = b"input";
    input.read(bytes);
}
