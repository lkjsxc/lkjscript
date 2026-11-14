// expect: reject E_TRAIT
fn pure<T: Data>(_value: T) {}
fn probe(listener: Listener) {
    pure(listener);
}
