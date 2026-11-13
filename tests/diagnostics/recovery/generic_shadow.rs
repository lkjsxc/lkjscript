use missing::*;
struct Known;
fn generic<T>(value: T) -> T { let _: () = true; value }
fn nominal(value: Known) { let _: Known = true; }
fn value() { let local = 42i64; local.absent(); }
