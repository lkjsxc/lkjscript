// expect: reject E_TYPE
// pending: Infer the empty slice element type from the method parameter, or suggest an explicitly typed empty array such as &[0u8; 0].
struct Receiver { value: usize }
impl Receiver { fn measure(&self, bytes: &[u8]) -> usize { bytes.len() } }
pub fn probe() -> usize { let receiver = Receiver { value: 0usize }; receiver.measure(&[]) }
