fn invalid<'a,'b,F:FnMut(&'a i64)>(mut f:F,value:&'b i64){ f(value); }
pub fn probe()->i64 {42}
