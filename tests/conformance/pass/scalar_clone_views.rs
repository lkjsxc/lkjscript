// expect: value 42
// witness: {}
#[derive(Clone)]
struct Fields<'a> {
    unit: (),
    flag: bool,
    character: char,
    byte: u8,
    real: f64,
    thin: &'a i64,
    text: &'a str,
    bytes: &'a [u8],
}
fn duplicate<T: Clone>(value: &T) -> T { value.clone() }
pub fn probe() -> i64 {
    let number = 42i64;
    let text = "λ🦀".to_string();
    let bytes = [3u8, 5u8, 7u8];
    let original = Fields {
        unit: (), flag: true, character: '水', byte: 255u8,
        real: f64::from_bits(9221120237041090561u64),
        thin: &number, text: text.as_str(), bytes: &bytes[..],
    };
    let copied = duplicate(&original);
    assert!(copied.unit == () && copied.flag && copied.character == '水');
    assert!(copied.byte == 255u8 && copied.real.to_bits() == 9221120237041090561u64);
    let thin = duplicate(&copied.thin);
    let text_copy = duplicate(&copied.text);
    let bytes_copy = duplicate(&copied.bytes);
    assert!(*thin == number && text_copy == "λ🦀");
    assert!(bytes_copy.len() == 3usize && bytes_copy[2usize] == 7u8);
    let array = [3u8, 5u8, 7u8].clone();
    let boxed = Box::new(11u8).clone();
    let vector = vec![13u8, 17u8].clone();
    assert!(array[1usize] == 5u8 && *boxed == 11u8 && vector[1usize] == 17u8);
    *thin
}
