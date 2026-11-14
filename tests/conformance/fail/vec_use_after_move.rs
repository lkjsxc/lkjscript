// expect: reject E_MOVE
// witness: {"rustAccept":false}
struct Ticket { value: i64 }
pub fn probe() -> i64 {
    let ticket = Ticket { value: 42 };
    let mut tickets: Vec<Ticket> = Vec::new();
    tickets.push(ticket);
    ticket.value
}
