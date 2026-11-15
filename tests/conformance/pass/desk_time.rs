// expect: value 42
// Desk local times are pure formatting with an explicitly supplied offset.
// Calendar boundaries independently checked against GNU date in UTC and JST-9.
use std::time::{local_minute, timezone_offset};
pub fn probe() -> i64 {
    assert!(local_minute(0i64, 0i64).unwrap() == "01-01 00:00");
    assert!(local_minute(-1i64, 0i64).unwrap() == "12-31 23:59");
    assert!(local_minute(0i64, 32400i64).unwrap() == "01-01 09:00");
    assert!(local_minute(951836400i64, 32400i64).unwrap() == "03-01 00:00");
    assert!(local_minute(978307200i64, -3600i64).unwrap() == "12-31 23:00");
    assert!(timezone_offset("UTC0").unwrap() == 0i64);
    assert!(timezone_offset("JST-9").unwrap() == 32400i64);
    assert!(timezone_offset("+05:45").unwrap() == 20700i64);
    assert!(timezone_offset("-03:30").unwrap() == -12600i64);
    assert!(timezone_offset("Europe/London").is_err());
    assert!(timezone_offset("+24:00").is_err());
    assert!(timezone_offset("+09:60").is_err());
    assert!(timezone_offset("+0x:00").is_err());
    assert!(local_minute(0i64, 86400i64).is_err());
    assert!(local_minute(9223372036854775807i64, 0i64).is_err());
    assert!(local_minute(-9223372036854775807i64 - 1i64, 0i64).is_err());
    42
}
