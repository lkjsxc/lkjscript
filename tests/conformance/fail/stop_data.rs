// expect: reject E_TRAIT
fn require<T: Data>(_value: T) {}
fn probe(stop: Stop) { require(stop); }
