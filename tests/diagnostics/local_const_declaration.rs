// expect: reject E_UNSUPPORTED
// diagnostic edits: Move the root-module literal declaration outside its function.
pub fn probe() -> i64 { const VALUE: i64 = 7; VALUE }
