struct Packet<A, B> { first: A, second: B }
type Alias<T> = Packet<T, i64>;
pub fn probe() -> i64 {
    let packet = Alias { first: 20i64, second: 22i64 };
    packet.first + packet.second
}
