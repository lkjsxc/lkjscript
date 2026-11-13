fn test<'a,F:FnMut(& &'a mut i64)>(value:&'a mut i64,mut f:F){let pointer=&mut *value;f(&pointer);drop(pointer);*value=42;}
pub fn probe()->i64{42}
