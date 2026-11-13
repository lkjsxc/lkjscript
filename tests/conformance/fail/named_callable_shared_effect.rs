fn invalid<'a,F:FnMut(&'a i64)>(mut f:F,value:&'a mut i64){ f(&*value); *value=0; }
pub fn probe()->i64 {42}
