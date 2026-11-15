// expect: reject E_BORROW
// witness: {"rustAccept":false,"rustCode":"E0596","world":true}
fn probe(child: &Child) { let _result = child.request_stop(); }
