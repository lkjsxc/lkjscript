#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Label<'a> {
    text: &'a str,
    ordinal: usize,
}
fn less<T: PartialOrd>(left: &T, right: &T) -> bool {
    left < right
}
pub fn probe() -> i64 {
    assert!("" < "a");
    assert!("ab" < "abc");
    assert!("a\0b" < "a\0c");
    assert!("é" < "雪");
    assert!("abc" <= "abc");
    assert!("abc" >= "abc");
    assert!(!("abc" > "abc"));
    let left = Label {
        text: "first",
        ordinal: 99,
    };
    let right = Label {
        text: "second",
        ordinal: 0,
    };
    assert!(less(&left, &right));
    assert!(less(&"first", &"second"));
    assert!(
        Label {
            text: "same",
            ordinal: 1
        } < Label {
            text: "same",
            ordinal: 2
        }
    );
    assert!(
        left == Label {
            text: "first",
            ordinal: 99
        }
    );
    assert_eq!(left.text, "first");
    42
}
