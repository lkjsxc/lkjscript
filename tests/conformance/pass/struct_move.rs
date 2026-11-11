struct Ticket { value: i64 }
fn redeem(ticket: Ticket) -> i64 { ticket.value }
pub fn probe() -> i64 {
    let ticket = Ticket { value: 42 };
    redeem(ticket)
}
