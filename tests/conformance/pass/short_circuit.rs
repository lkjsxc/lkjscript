pub fn probe() -> i64 {
    let zero: i64 = 0;
    if false && 1 / zero > 0 { return 0; }
    if true || 1 / zero > 0 { 42 } else { 0 }
}
