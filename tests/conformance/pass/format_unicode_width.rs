use std::fmt::Write;
pub fn probe() -> i64 {
    let value = format!(
        "{0:😀^7}|{0:水<5}|{1:🦀>6}|{2:#08x}|{3:+07}|{4:08X}|{5:b}|{5:o}|{6:>8}|{7}",
        "é水", 42i64, 42u32, -7i64, 42u8, -1i8, true, '水'
    );
    assert_text(
        value.as_str(),
        "😀😀é水😀😀😀|é水水水水|🦀🦀🦀🦀42|0x00002a|-000007|0000002A|11111111|377|    true|水",
    );
    let edges = format!("{}:{}", -9223372036854775808i64, 18446744073709551615u64);
    assert_text(edges.as_str(), "-9223372036854775808:18446744073709551615");
    let mut output = String::new();
    writeln!(output, "{:💧^5}", '水').unwrap();
    writeln!(&mut output).unwrap();
    assert_text(output.as_str(), "💧💧水💧💧\n\n");
    42
}
fn assert_text(actual: &str, expected: &str) {
    let a = actual.as_bytes();
    let b = expected.as_bytes();
    assert!(a.len() == b.len());
    let mut index = 0usize;
    while index < a.len() {
        assert!(a[index] == b[index]);
        index = index + 1;
    }
}
