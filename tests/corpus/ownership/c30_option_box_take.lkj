struct N { next: Option<Box<N>>, v: i64 }
fn pop(head: &mut Option<Box<N>>) -> i64 { let old = std::mem::replace(head, None); match old { Some(b) => { let n: N = *b; *head = n.next; n.v } None => 0 } }
pub fn probe() -> i64 { let mut h: Option<Box<N>> = Some(Box::new(N { next: Some(Box::new(N { next: None, v: 2 })), v: 40 })); pop(&mut h) + pop(&mut h) + pop(&mut h) }
