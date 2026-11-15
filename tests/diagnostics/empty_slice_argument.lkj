// expect: reject E_TYPE
// diagnostic edits: Give this empty &[u8] argument a typed zero-length array.
struct Receiver { value: usize }
impl Receiver { fn measure(&self, bytes: &[u8]) -> usize { bytes.len() } }
pub fn probe() -> usize { let receiver = Receiver { value: 0usize }; receiver.measure(&[]) }
