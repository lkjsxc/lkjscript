#[derive(Clone, Copy)]
struct Leaf {
    bytes: [u8; 3],
    value: i64,
}
#[derive(Clone, Copy)]
enum Choice {
    Empty,
    Leaf(Leaf),
}
#[derive(Copy)]
struct NoClone {
    value: i64,
}
impl Clone for NoClone {
    fn clone(&self) -> Self {
        panic!("array repeat must not call Clone");
    }
}
fn seed(calls: &mut i64) -> Leaf {
    *calls += 1;
    Leaf {
        bytes: [1u8, 2u8, 3u8],
        value: 7,
    }
}
pub fn probe() -> i64 {
    let mut calls = 0i64;
    let mut leaves = [seed(&mut calls); 5];
    assert!(calls == 1);
    leaves[0].bytes[1] = 99u8;
    leaves[0].value = 88;
    assert!(leaves[1].bytes[1] == 2u8 && leaves[4].value == 7);
    let mut matrix = [[leaves[1]; 2]; 3];
    matrix[0][0].bytes[0] = 55u8;
    assert!(matrix[0][1].bytes[0] == 1u8);
    assert!(matrix[1][0].bytes[0] == 1u8);
    assert!(leaves[1].bytes[0] == 1u8);
    let mut choices = [Choice::Leaf(leaves[1]); 3];
    match &mut choices[0] {
        Choice::Leaf(value) => value.bytes[2] = 77u8,
        Choice::Empty => panic!(),
    }
    match &choices[2] {
        Choice::Leaf(value) => assert!(value.bytes[2] == 3u8),
        Choice::Empty => panic!(),
    }
    let absent = [Choice::Empty; 2];
    assert!(matches!(absent[1], Choice::Empty));
    let bytes = [7u8; 8192];
    assert!(bytes.len() == 8192 && bytes[0] == 7u8 && bytes[8191] == 7u8);
    let chars = ['水'; 3];
    assert!(chars[2] == '水');
    let flags = [true; 3];
    assert!(flags[2]);
    let numbers = [1.5f64; 3];
    assert!(numbers[2] == 1.5f64);
    let copies = [NoClone { value: 42 }; 3];
    copies[2].value
}
