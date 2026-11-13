fn fresh<F:FnMut(&mut i64)>(f:&mut F){let mut local=1;f(&mut local)}
fn invalid<'a,F:FnMut(&'a mut i64)>(mut f:F){fresh(&mut f);}
pub fn probe()->i64{42}
