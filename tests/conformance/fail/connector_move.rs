// expect: reject E_MOVE
// witness: {"rustAccept":false,"rustCode":"E0382","world":true}
fn consume(_endpoint: Connector) {}
fn probe(endpoint: Connector) {
    consume(endpoint);
    consume(endpoint);
}
