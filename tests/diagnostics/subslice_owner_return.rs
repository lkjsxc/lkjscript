// expect: reject E_BORROW
// rewrite: Compute the slice length in a nested block before assigning the scalar and returning its owner.
struct Message { bytes: Vec<u8>, length: usize }
pub fn probe() -> Message {
    let mut message = Message { bytes: vec![1u8, 2u8], length: 0usize };
    let tail = &message.bytes[1usize..];
    message.length = tail.len();
    message
}
