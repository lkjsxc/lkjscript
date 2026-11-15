// expect: value 42
use std::str::trim_ascii;

pub fn probe() -> i64 {
    assert!(trim_ascii("") == "");
    assert!(trim_ascii(" \t\n\u{b}\u{c}\r ") == "");
    assert!(trim_ascii("plain") == "plain");
    assert!(trim_ascii("\t\n 3600 \r\n") == "3600");
    assert!(trim_ascii(" left") == "left");
    assert!(trim_ascii("right ") == "right");
    assert!(trim_ascii(" a \t b ") == "a \t b");
    assert!(trim_ascii("\u{0} x \u{0}") == "\u{0} x \u{0}");
    assert!(trim_ascii("\u{a0}text\u{a0}") == "\u{a0}text\u{a0}");
    assert!(trim_ascii(" \u{a0}text\u{a0}\t") == "\u{a0}text\u{a0}");
    let owned = " \t雪🙂\r\n".to_string();
    let view = trim_ascii(owned.as_str());
    assert!(view == "雪🙂");
    assert!(owned == " \t雪🙂\r\n");
    42
}
