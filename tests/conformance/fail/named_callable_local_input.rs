fn invalid<'a,F:FnMut(&'a mut i64)>(mut f:F){ let mut value=42; f(&mut value); }
pub fn probe()->i64 {42}
