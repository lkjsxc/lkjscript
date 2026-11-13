trait Value { type Item; fn get(&self) -> Self::Item; }
struct Number { value: i64 }
impl Value for Number { type Item=i64; fn get(&self)->i64 { self.value } }
struct Map<I,F> { input:I, function:F }
impl<F:FnMut(I::Item)->i64,I:Value> Map<I,F> {
    fn run(&mut self)->i64 { (self.function)(self.input.get()) }
}
fn apply<F:FnMut(I::Item)->i64,I:Value>(input:I,mut function:F)->i64 {
    function(input.get())
}
pub fn probe()->i64 {
    let mut offset=1i64;
    let mut map=Map { input:Number { value:20 }, function:|value:i64| { offset=offset+1; value+offset } };
    map.run()+apply(Number { value:20 },|value:i64|value)
}
