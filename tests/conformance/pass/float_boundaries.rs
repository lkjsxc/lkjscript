pub fn probe() -> i64 {
    let zero = 0.0f64;
    let nan = zero / zero;
    let infinity = 1.0f64 / zero;
    let negative_zero = -zero;
    assert!(nan != nan);
    assert!(!(nan == zero));
    assert!(!(nan < zero));
    assert!(!(nan <= zero));
    assert!(!(nan > zero));
    assert!(!(nan >= zero));
    assert!(zero == negative_zero);
    assert!(1.0f64 / negative_zero < zero);
    assert!((nan as i64) == 0i64 && (nan as u64) == 0u64);
    assert!((infinity as i64) == 9223372036854775807i64);
    assert!((-infinity as i64) == -9223372036854775808i64);
    assert!((infinity as u64) == 18446744073709551615u64);
    assert!((-infinity as u64) == 0u64);
    assert!((infinity as i32) == 2147483647i32);
    assert!((-3.5f64 as u8) == 0u8);
    assert!((9223372036854774784.0f64 as u64) == 9223372036854774784u64);
    assert!((9223372036854775808.0f64 as u64) == 9223372036854775808u64);
    assert!((18446744073709549568.0f64 as u64) == 18446744073709549568u64);
    assert!((18446744073709551616.0f64 as u64) == 18446744073709551615u64);
    assert!((18446744073709551615u64 as f64) == 18446744073709551616.0f64);
    assert!((9223372036854775807i64 as f64) == 9223372036854775808.0f64);
    assert!((-9223372036854775808i64 as f64) == -9223372036854775808.0f64);
    assert!((9007199254740993u64 as f64) == 9007199254740992.0f64);
    assert!(1.5f64 + 2.25f64 == 3.75f64);
    assert!(1.5f64 - 2.25f64 == -0.75f64);
    assert!(1.5f64 * 2.0f64 == 3.0f64);
    assert!(3.0f64 / 2.0f64 == 1.5f64);
    42i64
}
