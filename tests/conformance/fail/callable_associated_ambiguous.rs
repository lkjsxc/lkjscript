trait First { type Item; }
trait Second { type Item; }
fn invalid<F:Fn(I::Item)->i64,I:First+Second>(value:I,function:F)->i64 { 42 }
pub fn probe()->i64 { 42 }
