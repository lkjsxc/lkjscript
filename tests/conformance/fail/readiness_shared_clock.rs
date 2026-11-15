// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0308","world":true}
fn probe(ready: &mut Readiness, clock: &Clock) {
    let _events = ready.wait(clock, 0usize);
}
