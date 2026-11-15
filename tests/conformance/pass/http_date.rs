// expect: value 42
// IMF-fixdate grammar and semantic field requirements:
// https://www.rfc-editor.org/rfc/rfc9110.html#section-5.6.7
// https://www.rfc-editor.org/rfc/rfc5322.html#section-3.3
// Unix timestamps and calendar boundaries independently checked with GNU date.
use std::time::http_date;

fn vector(seconds: i64, expected: &str) {
    let actual = http_date(seconds).unwrap();
    assert!(actual == expected);
    assert!(actual.len() == 29usize);
}
pub fn probe() -> i64 {
    vector(784111777, "Sun, 06 Nov 1994 08:49:37 GMT");
    vector(0, "Thu, 01 Jan 1970 00:00:00 GMT");
    vector(-1, "Wed, 31 Dec 1969 23:59:59 GMT");
    vector(-86400, "Wed, 31 Dec 1969 00:00:00 GMT");
    vector(-86401, "Tue, 30 Dec 1969 23:59:59 GMT");
    vector(86400, "Fri, 02 Jan 1970 00:00:00 GMT");
    vector(172800, "Sat, 03 Jan 1970 00:00:00 GMT");
    vector(259200, "Sun, 04 Jan 1970 00:00:00 GMT");
    vector(345600, "Mon, 05 Jan 1970 00:00:00 GMT");
    vector(432000, "Tue, 06 Jan 1970 00:00:00 GMT");
    vector(518400, "Wed, 07 Jan 1970 00:00:00 GMT");
    vector(-2208988800, "Mon, 01 Jan 1900 00:00:00 GMT");
    vector(-2203891201, "Wed, 28 Feb 1900 23:59:59 GMT");
    vector(-2203891200, "Thu, 01 Mar 1900 00:00:00 GMT");
    vector(951868799, "Tue, 29 Feb 2000 23:59:59 GMT");
    vector(978307199, "Sun, 31 Dec 2000 23:59:59 GMT");
    vector(978307200, "Mon, 01 Jan 2001 00:00:00 GMT");
    vector(4107542399, "Sun, 28 Feb 2100 23:59:59 GMT");
    vector(4107542400, "Mon, 01 Mar 2100 00:00:00 GMT");
    vector(13574608496, "Tue, 29 Feb 2400 12:34:56 GMT");
    vector(2147483647, "Tue, 19 Jan 2038 03:14:07 GMT");
    vector(2147483648, "Tue, 19 Jan 2038 03:14:08 GMT");
    vector(253402300799, "Fri, 31 Dec 9999 23:59:59 GMT");
    assert!(http_date(-2208988801).is_err());
    assert!(http_date(253402300800).is_err());
    assert!(http_date(9223372036854775807i64).is_err());
    assert!(http_date(-9223372036854775807i64 - 1i64).is_err());
    42
}
