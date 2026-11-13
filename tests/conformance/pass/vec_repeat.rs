#[derive(Clone)]
struct V { value: i64 }
fn repeat<T:Clone>(x:T,n:usize)->Vec<T>{vec![x;n]}
pub fn probe()->i64 {let values=vec![21i64;2usize];let empty=vec![Box::new(12i64);0usize];let v=repeat(V{value:14},3); assert!(empty.len()==0);values[0]+v[2].value+7}
