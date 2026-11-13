fn invalid<'a,'b,F:FnOnce((&'a i64,&'b i64))>(f:F,first:&'a i64,second:&'a i64){f((first,second));}
pub fn probe()->i64 {42}
