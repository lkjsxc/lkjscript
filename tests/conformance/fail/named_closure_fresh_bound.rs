fn helper<F:FnMut(&mut i64)>(mut f:F,value:&mut i64){f(value);f(value);} fn test<'a,T>(value:&'a mut i64,unused:T){drop(unused);helper(|input:&'a mut i64|{*input=42;},value);}
pub fn probe()->i64{42}
