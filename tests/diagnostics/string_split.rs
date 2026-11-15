// expect: reject E_UNSUPPORTED
// diagnostic edits: Borrow String as str once; replace the unavailable count with the source fold operation.
pub fn probe() -> usize { "a\nb".to_string().split('\n').count() }
