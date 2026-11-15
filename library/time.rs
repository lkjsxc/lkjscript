//! HTTP date formatting from signed Unix seconds, with no ambient locale/zone.
//! https://www.rfc-editor.org/rfc/rfc9110.html#section-5.6.7
//! https://www.rfc-editor.org/rfc/rfc5322.html#section-3.3
use crate::{Result, String};
use crate::result::Result::{Err, Ok};

fn weekday(day: i64) -> &'static str {
    match day {
        0 => "Sun", 1 => "Mon", 2 => "Tue", 3 => "Wed",
        4 => "Thu", 5 => "Fri", _ => "Sat",
    }
}
fn month_name(month: i64) -> &'static str {
    match month {
        1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
        5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
        9 => "Sep", 10 => "Oct", 11 => "Nov", _ => "Dec",
    }
}
fn month_days(month: i64, leap: bool) -> i64 {
    match month {
        2 => if leap { 29i64 } else { 28i64 },
        4 | 6 | 9 | 11 => 30i64,
        _ => 31i64,
    }
}

/// IMF-fixdate uses exactly four year digits and inherits the IMF requirement
/// that years be at least 1900. Out-of-range seconds return an error, including
/// both i64 extremes, before any calendar arithmetic. Unix days have 86400
/// seconds; this formatter never invents a leap-second :60.
pub fn http_date(seconds: i64) -> Result<String, String> {
    if seconds < -2_208_988_800i64 || seconds > 253_402_300_799i64 {
        return Err("HTTP date requires a UTC year from 1900 through 9999".to_string());
    }
    let mut day = seconds / 86_400i64;
    let mut within_day = seconds % 86_400i64;
    if within_day < 0i64 {
        day -= 1i64;
        within_day += 86_400i64;
    }
    let name = weekday(((day + 4i64) % 7i64 + 7i64) % 7i64);
    // 719162 days separate Gregorian 0001-01-01 and Unix 1970-01-01.
    // A 400-year cycle has 146097 days. Its last century and final four-year
    // cycle carry the leap day, hence the capped quotient at those boundaries.
    let total = day + 719_162i64;
    let mut year = 1i64 + (total / 146_097i64) * 400i64;
    let mut remaining = total % 146_097i64;
    let mut centuries = remaining / 36_524i64;
    if centuries == 4i64 { centuries = 3i64; }
    year += centuries * 100i64;
    remaining -= centuries * 36_524i64;
    let fours = remaining / 1_461i64;
    year += fours * 4i64;
    remaining -= fours * 1_461i64;
    let mut years = remaining / 365i64;
    if years == 4i64 { years = 3i64; }
    year += years;
    remaining -= years * 365i64;
    let leap = year % 4i64 == 0i64 && (year % 100i64 != 0i64 || year % 400i64 == 0i64);
    let mut month = 1i64;
    loop {
        let size = month_days(month, leap);
        if remaining < size { break; }
        remaining -= size;
        month += 1i64;
    }
    let day = remaining + 1i64;
    let month = month_name(month);
    let hour = within_day / 3600i64;
    let minute = (within_day % 3600i64) / 60i64;
    let second = within_day % 60i64;
    Ok(format!("{name}, {day:02} {month} {year:04} {hour:02}:{minute:02}:{second:02} GMT"))
}
