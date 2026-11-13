fn mutable_window<'a>(bytes: &'a mut [u8]) -> &'a mut [u8] {
    &mut bytes[1..3]
}
fn choose<'a>(bytes: &'a [u8], first: bool) -> &'a [u8] {
    if first {
        &bytes[..2]
    } else {
        &bytes[1..3]
    }
}
#[derive(Clone, Copy)]
struct Views<'a> {
    bytes: &'a [u8],
    text: &'a str,
}
pub fn probe() -> i64 {
    let values = vec![1u8, 19, 23, 7];
    let slice = choose(&values, false);
    let mut iterator = slice.iter();
    let first = iterator.next().unwrap();
    let second = iterator.next().unwrap();
    drop(iterator);
    assert!(*first == 19 && *second == 23);
    let captured = || &slice[1..];
    let returned = captured();
    drop(captured);
    assert!(returned.len() == 1 && returned[0] == 23);
    let text = "aλ🦀z";
    let views = Views {
        bytes: slice,
        text: &text[1..7],
    };
    let copy = views.clone();
    assert!(copy.bytes.len() == 2 && copy.text.as_bytes()[0] == 206);
    let raw = vec![255u8, 206, 187, 255];
    let valid = std::str::from_utf8(&raw[1..3]).unwrap();
    assert!(valid.len() == 2 && valid.as_bytes()[0] == 206);
    let nested = &copy.text[2..];
    let decoded = std::str::from_utf8(nested.as_bytes()).unwrap();
    assert!(decoded.len() == 4 && decoded.as_bytes()[3] == 128);
    assert!(std::str::from_utf8(&raw[2..3]).is_err());
    let chars = ['a', 'λ', '🦀', 'z'];
    let part = &chars[1..3];
    assert!(part.len() == 2 && part[1..][0] == '🦀');
    let shorts = [1u16, 19, 23, 7];
    let part = &shorts[1..3];
    assert!(part[0] + part[1] == 42);
    let mut writable = vec![0u8, 0, 0, 0];
    let part = mutable_window(&mut writable);
    part[0] = 40;
    part[1] = 2;
    drop(part);
    writable[1] as i64 + writable[2] as i64
}
