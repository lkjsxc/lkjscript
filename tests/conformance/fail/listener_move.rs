// expect: reject E_MOVE
// witness: {"rustAccept":false,"rustCode":"E0382","world":true}
fn consume(_listener: Listener) {}
fn probe(listener: Listener) {
    consume(listener);
    consume(listener);
}
