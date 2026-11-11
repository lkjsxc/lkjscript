enum Packet { Empty, Data(Box<i64>, i64) }
pub fn probe() -> i64 {
    let packet = Packet::Data(Box::new(20), 22);
    match packet {
        Packet::Empty => 0,
        Packet::Data(owner, scalar) => *owner + scalar,
    }
}
