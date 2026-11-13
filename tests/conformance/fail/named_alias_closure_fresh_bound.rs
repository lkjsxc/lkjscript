type Input<'a>=&'a mut i64; fn helper<F:FnMut(&mut i64)>(mut f:F,value:&mut i64){f(value);f(value);} fn test<'a>(value:&'a mut i64){helper(|input:Input<'a>|{*input=42;},value);}
pub fn probe()->i64{42}
