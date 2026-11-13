fn helper<'b,F:FnMut(&'b mut i64)>(f:&mut F,x:&'b mut i64){f(x)}
fn invalid<'a,F:FnMut(&'a mut i64)>(mut f:F,x:&'a mut i64){helper(&mut f,x);*x=0;}
pub fn probe()->i64{42}
