enum Packet<T> {
    Empty,
    Data { first: T, second: T },
}

pub fn probe() -> i64 {
    let mut packet = Packet::Data {
        second: 20i64,
        first: 1i64,
    };
    match &mut packet {
        Packet::Data { first, second } => {
            *first = *first + 1;
            *second = *second * 2;
        }
        Packet::Empty => {}
    }
    match packet {
        Packet::Data { first, second } => first + second,
        Packet::Empty => 0,
    }
}
