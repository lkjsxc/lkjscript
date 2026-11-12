struct Stack<T> { items: Vec<T> }
impl<T> Stack<T> { fn new() -> Stack<T> { Stack { items: Vec::new() } } fn push(&mut self, t: T) { self.items.push(t); } fn pop(&mut self) -> Option<T> { self.items.pop() } }
pub fn probe() -> i64 { let mut s: Stack<i64> = Stack::new(); s.push(42); match s.pop() { Some(v) => v, None => 0 } }
