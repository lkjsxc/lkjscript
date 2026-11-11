struct Packet { value: i64 }
fn send<T: Send>(value: T) -> T { value }
fn sync<T: Sync>(value: &T) -> &T { value }
pub fn probe() -> i64 {
    let packet = send(Packet { value: 42 });
    sync(&packet).value
}
