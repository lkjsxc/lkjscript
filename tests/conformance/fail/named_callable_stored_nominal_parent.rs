struct Wrapped<'a>{value:&'a mut i64} fn test<'a,F:FnMut(&Wrapped<'a>)>(value:&'a mut i64,mut f:F){let wrapped=Wrapped{value:&mut *value};f(&wrapped);drop(wrapped);drop(f);*value=42;}
pub fn probe()->i64{42}
