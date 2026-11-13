fn helper<'b,F:FnOnce(&'b mut i64)>(f:F,x:&'b mut i64)->&'b mut i64{drop(f);x}
fn invalid<'a,F:FnOnce(&'a mut i64)>(f:F,x:&'a mut i64)->i64{let result=helper(f,x);drop(result);*x=0;*x}
pub fn probe()->i64{42}
