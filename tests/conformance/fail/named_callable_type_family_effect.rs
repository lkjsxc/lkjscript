fn helper<A,F:FnMut(A)>(mut f:F,value:A){f(value)}
fn invalid<'a,F:FnMut(&'a mut i64)>(f:F,value:&'a mut i64){helper(f,&mut *value);*value=0;}
pub fn probe()->i64{42}
