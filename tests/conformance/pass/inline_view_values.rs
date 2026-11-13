#[derive(Clone, Copy)]
struct Views<'a> {
    first: &'a [u8],
    second: &'a [u8],
}

fn choose<'a>(before: i64, left: &'a [u8], flag: bool, right: &'a [u8], after: i64) -> &'a [u8] {
    assert!(before == 19 && after == 23);
    if flag { left } else { right }
}
fn from_slot<'a>(slot: &&'a [u8]) -> &'a [u8] { *slot }
fn text_slot<'a>(slot: &&'a str) -> &'a str { *slot }
fn mutable<'a>(view: &'a mut [i64]) -> &'a mut [i64] { view }
pub fn text() -> &'static str { "λ🦀" }

pub fn probe() -> i64 {
    let left = vec![1u8, 2];
    let right = vec![5u8, 7, 11, 13, 17];
    let selected = choose(19, &left, false, &right, 23);
    assert!(selected.len() == 5 && selected[4] == 17);
    let short = choose(19, &left, true, &right, 23);
    assert!(short.len() == 2 && short[1] == 2);
    let views = Views { first: short, second: selected };
    let copied = views;
    let cloned = views.clone();
    assert!(copied.first.len() == 2 && copied.second.len() == 5);
    assert!(from_slot(&cloned.second)[3] == 13);
    let optional = Some(selected);
    assert!(from_slot(optional.as_ref().unwrap())[4] == 17);
    let capture = selected;
    let get = || capture;
    assert!(get().len() == 5 && get()[2] == 11);
    let message = text();
    assert!(message.len() == 6 && text_slot(&message).as_bytes()[0] == 206);
    let mut numbers = vec![1i64, 2, 3];
    let writable: &mut [i64] = &mut numbers;
    let writable = mutable(writable);
    assert!(writable.len() == 3);
    writable[0] = 40;
    writable[2] = 2;
    drop(writable);
    numbers[0] + numbers[2]
}
