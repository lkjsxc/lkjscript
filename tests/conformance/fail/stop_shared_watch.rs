// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0308","world":true}
fn probe(ready: &mut Readiness, stop: &Stop) { let _result = ready.watch_stop(stop, 1usize); }
