// expect: reject E_MOVE
// witness: {"rustAccept":false,"rustCode":"E0382","world":true}
fn consume(_file: File) {}
fn probe(file: File) {
    consume(file);
    consume(file);
}
