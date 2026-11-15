// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0308","world":true}
fn probe(entropy: &mut Entropy, bytes: &[u8]) { let _result = entropy.fill(bytes); }
