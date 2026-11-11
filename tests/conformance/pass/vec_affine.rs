struct Ticket { value: i64 }
pub fn probe() -> i64 {
    let mut tickets: Vec<Ticket> = Vec::new();
    tickets.push(Ticket { value: 42 });
    match tickets.pop() { Some(ticket) => ticket.value, None => 0 }
}
