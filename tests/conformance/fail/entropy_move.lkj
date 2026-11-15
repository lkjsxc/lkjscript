// expect: reject E_MOVE
// witness: {"rustAccept":false,"rustCode":"E0382","world":true}
fn consume(_entropy: Entropy) {}
fn probe(entropy: Entropy) { consume(entropy); consume(entropy); }
