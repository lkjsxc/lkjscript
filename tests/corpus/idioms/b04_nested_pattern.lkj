enum E { A(Option<i64>), B }
pub fn probe() -> i64 { let e = E::A(Some(42)); match e { E::A(Some(v)) => v, E::A(None) => 1, E::B => 2 } }
