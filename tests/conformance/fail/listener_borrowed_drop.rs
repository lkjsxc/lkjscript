// expect: reject E_BORROW
// witness: {"rustAccept":false,"rustCode":"E0505","world":true}
fn probe(mut listener: Listener) {
    let loan = &mut listener;
    drop(listener);
    let _port = loan.port();
}
