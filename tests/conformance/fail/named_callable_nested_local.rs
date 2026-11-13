struct View<'a>{ value:&'a i64 }
fn invalid<'a,F:FnOnce(View<'a>)>(f:F){ let value=42; f(View{value:&value}); }
pub fn probe()->i64 {42}
