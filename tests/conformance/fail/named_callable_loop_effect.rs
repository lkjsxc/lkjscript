fn invalid<'a,F:FnMut(&'a i64)>(mut f:F,x:&'a mut i64){let mut n=0;while n<3{*x=0;f(&*x);n=n+1;}}
pub fn probe()->i64{42}
