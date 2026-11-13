fn invalid<'a,F:FnOnce((&'a i64,i64))>(f:F,value:&'a mut i64){ f((&*value,42)); *value=0; }
pub fn probe()->i64 {42}
