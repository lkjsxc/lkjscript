// expect: reject E_BORROW
// pending: Name the slice temporary and explain that its lexical scope prevents returning the owner; suggest computing the scalar result in a nested block.
struct Message { bytes: Vec<u8>, length: usize }
pub fn probe() -> Message {
    let mut message = Message { bytes: vec![1u8, 2u8], length: 0usize };
    let tail = &message.bytes[1usize..];
    message.length = tail.len();
    message
}
