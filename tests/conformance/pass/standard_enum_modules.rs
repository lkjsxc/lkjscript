use std::option::Option as Maybe;
use std::option::Option::{None as Absent, Some as Present};
use std::result::Result as Outcome;
use std::result::Result::{Err as Bad, Ok as Good};
use std::result::Result::{Err as No, Ok as Yes};
fn value(input: Maybe<Outcome<i64, i64>>) -> i64 {
    match input {
        Present(Yes(value)) => value,
        Present(No(error)) => error,
        Absent => 0,
    }
}
pub fn probe() -> i64 {
    let original: Option<Result<i64, i64>> = Some(Ok(42));
    let canonical: Maybe<Outcome<i64, i64>> = original;
    let unused: Outcome<i64, i64> = Good(0);
    let failure: Outcome<i64, i64> = Bad(1);
    assert!(unused.is_ok() && failure.is_err());
    value(canonical)
}
