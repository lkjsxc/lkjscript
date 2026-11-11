struct Packet<A, B> { first: A, second: B }
type Tied<T> = Packet<T, T>;
pub fn probe() -> i64 {
    let packet = Tied { first: 20i64, second: true };
    0
}
