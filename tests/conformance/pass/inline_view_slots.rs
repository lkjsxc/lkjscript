#[derive(Clone, Copy)]
struct Mixed<'a> {
    head: u8,
    bytes: &'a [u8],
    marker: char,
    text: &'a str,
    tail: u16,
}

#[derive(Clone, Copy)]
enum Either<'a> {
    Bytes(&'a [u8]),
    Text(&'a str),
}

fn recursive<'a>(bytes: &'a [u8], depth: usize) -> &'a [u8] {
    if depth == 0 {
        bytes
    } else {
        let result = recursive(bytes, depth - 1);
        assert!(result.len() == bytes.len());
        result
    }
}

fn many<'a>(a: u8, bytes: &'a [u8], b: u16, text: &'a str, c: char, other: &'a [u8], d: i64) -> &'a [u8] {
    assert!(a == 3 && b == 5 && c == 'λ' && d == 7);
    if text.len() == 2 { bytes } else { other }
}

fn reborrow_slot(slot: &mut &mut [i64]) {
    let inner: &mut [i64] = &mut **slot;
    inner[0] = 40;
    inner[1] = 2;
}

fn select_mut<'a>(left: &'a mut [i64], right: &'a mut [i64], choose_left: bool) -> &'a mut [i64] {
    if choose_left { left } else { right }
}

pub fn probe() -> i64 {
    let first = [19u8, 23];
    let second = [2u8, 3, 5, 7, 11];
    let short: &[u8] = &first;
    let long: &[u8] = &second;
    let original = Mixed { head: 3, bytes: short, marker: 'λ', text: "ok", tail: 5 };
    let copied = original;
    let mut cloned = original.clone();
    let old_text = std::mem::replace(&mut cloned.text, "λ🦀");
    let old_bytes = std::mem::replace(&mut cloned.bytes, long);
    assert!(old_text.len() == 2 && cloned.text.len() == 6);
    assert!(old_bytes.len() == 2 && old_bytes[1] == 23);
    assert!(cloned.bytes.len() == 5 && cloned.bytes[4] == 11);
    assert!(copied.head == 3 && copied.marker == 'λ' && copied.tail == 5);
    let selected = many(3, short, 5, "λ", 'λ', long, 7);
    assert!(recursive(selected, 64).len() == 2);
    let mut selected = selected;
    let previous = std::mem::replace(&mut selected, recursive(long, 64));
    assert!(previous[1] == 23 && selected.len() == 5 && selected[4] == 11);
    let first_case = Either::Bytes(selected);
    let second_case = Either::Text(cloned.text);
    match first_case.clone() { Either::Bytes(bytes) => assert!(bytes.len() == 5), _ => panic!() }
    match second_case { Either::Text(text) => assert!(text.as_bytes()[0] == 206), _ => panic!() }
    let chars = ['λ', '🦀', 'x'];
    let chars: &[char] = &chars;
    assert!(chars.len() == 3 && chars[1] == '🦀');
    let mut left = [1i64, 2];
    let mut right = [3i64, 4];
    let mut writable = select_mut(&mut left, &mut right, false);
    reborrow_slot(&mut writable);
    drop(writable);
    assert!(left[0] == 1 && right[1] == 2);
    right[0] + right[1]
}
