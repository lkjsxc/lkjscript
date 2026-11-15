// expect: trap R0004
// Contract: unwrap reports its ordinary library callee; preserve errno and the user location with an explicit match and an already granted writer.
pub fn probe() { let failure: Result<(),i64> = Err(5i64); failure.unwrap(); }
