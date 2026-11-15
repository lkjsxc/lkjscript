// expect: reject E_MOVE
// witness: {"rustAccept":false,"rustCode":"E0382","world":true}
fn consume(_ready: Readiness) {}
fn probe(ready: Readiness) {
    consume(ready);
    consume(ready);
}
