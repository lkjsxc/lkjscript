enum Packet { Data { value: i64 } }
pub fn probe() -> i64 {
    let packet = Packet::Data(42i64);
    0
}
