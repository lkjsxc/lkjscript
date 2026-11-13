struct View<'a>{ value:&'a i64 }
fn invalid<'a,F:FnOnce(View<'a>)>(f:F,value:&'a mut i64){ f(View{value:&*value}); *value=0; }
pub fn probe()->i64 {42}
