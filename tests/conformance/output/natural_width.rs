#[derive(Clone)]
struct Packed { byte: u8, short: i16, word: u32, owned: Box<i8>, flag: bool, tail: u8 }
#[derive(Clone)]
enum Choice { Mixed(u8, i16, Box<i8>, bool), Word(u32) }
fn read(value: Choice) -> i64 {
    match value {
        Choice::Mixed(a, b, c, d) => { assert!(d); a as i64 + b as i64 + *c as i64 },
        Choice::Word(n) => n as i64,
    }
}
pub fn main(mut world: World) {
    let mut out = world.stdout();
    let mut bytes = [1u8, 2u8, 255u8, 4u8];
    let old = std::mem::replace(&mut bytes[1usize], 254u8);
    assert!(old == 2u8 && bytes[0usize] == 1u8 && bytes[2usize] == 255u8);
    out.write(&bytes);
    let mut signed = [-128i8, -1i8, 0i8, 127i8];
    assert!(signed[0usize] == -128i8);
    signed[1usize] = -27i8;
    assert!(signed[0usize] == -128i8 && signed[2usize] == 0i8);
    let mut short = -30000i16;
    let old_short = std::mem::replace(&mut short, 12345i16);
    assert!(old_short == -30000i16 && short == 12345i16);
    let mut packed = Packed { byte: 255u8, short: -32768i16, word: 4294967295u32, owned: Box::new(-100i8), flag: true, tail: 42u8 };
    let saved = packed.clone();
    packed.byte = 1u8;
    packed.short = -123i16;
    packed.tail = 9u8;
    assert!(saved.byte == 255u8 && saved.short == -32768i16 && saved.word == 4294967295u32);
    assert!(*saved.owned == -100i8 && saved.flag && saved.tail == 42u8);
    assert!(packed.byte == 1u8 && packed.short == -123i16 && packed.tail == 9u8);
    let choice = Choice::Mixed(250u8, -200i16, Box::new(-8i8), true);
    assert!(read(choice.clone()) == 42i64);
    assert!(read(choice) == 42i64 && read(Choice::Word(42u32)) == 42i64);
    let mut values: Vec<i16> = Vec::new();
    let mut i = 0i16;
    while i < 100i16 { values.push(i - 50i16); i += 1i16; }
    let copy = values.clone();
    assert!(values.pop().unwrap() == 49i16 && values.pop().unwrap() == 48i16);
    assert!(copy[0usize] == -50i16 && copy[99usize] == 49i16);
    let small = Box::new(-128i8);
    assert!(*small.clone() == -128i8 && *small == -128i8);
    let mut bits: Vec<bool> = Vec::new();
    bits.push(true); bits.push(false); bits.push(true);
    assert!(bits.pop().unwrap() && !bits.pop().unwrap() && bits.pop().unwrap());
    out.write("layout\n".as_bytes());
}
