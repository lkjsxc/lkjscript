// expect: value 42
#[derive(Clone, Copy)]
struct Pair { left: i64, right: i64, extra: [i64; 2] }
struct Owned { words: [i64; 20], text: String }
fn pair(value: i64) -> Pair {
    Pair { left: value, right: value + 1, extra: [value + 2, value + 3] }
}
fn inspect(snapshot: Pair, changing: &mut Pair) -> i64 {
    changing.left += 100;
    snapshot.left + snapshot.right
}
fn chain(value: i64) -> i64 {
    let first = pair(value);
    let second = first;
    let third = second;
    third.left + third.right + third.extra[0] + third.extra[1]
}
fn branches(change: bool) -> i64 {
    let original = pair(3);
    let mut copied = original;
    if change { copied.left = 20; } else { copied.right = 40; }
    original.left + original.right + copied.left + copied.right
}
pub fn probe() -> i64 {
    assert!(chain(5) == 26);
    assert!(branches(true) == 31);
    assert!(branches(false) == 50);
    let mut original = pair(10);
    let snapshot = original;
    original.left = 90;
    assert!(snapshot.left == 10);
    assert!(original.left == 90);
    let saved = original;
    assert!(inspect(saved, &mut original) == 101);
    assert!(original.left == 190);
    assert!(saved.left == 90);
    let mut carry = pair(1);
    let mut total = 0;
    let mut index = 0;
    while index < 8 {
        let snapshot = carry;
        let mut next = snapshot;
        next.left += 1;
        total += snapshot.left;
        carry = next;
        index += 1;
    }
    assert!(total == 36);
    assert!(carry.left == 9);
    let first = pair(20);
    let mut second = first;
    { let view = &second; assert!(view.right == 21); }
    second.left += 3;
    assert!(second.left == 23);
    let first = Owned { words: [7; 20], text: "alive".to_string() };
    let second = first;
    let third = second;
    assert!(third.words[19] == 7);
    assert!(third.text == "alive");
    42
}
