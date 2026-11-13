fn helper<'b,F:FnMut(&'b i64)>(f:&mut F,x:&'b i64){f(x)}
fn invalid<'a,F:FnMut(&'a i64)>(mut f:F){let local=1;helper(&mut f,&local);}
pub fn probe()->i64{42}
