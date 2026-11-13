struct Mark { value:i64 }
impl Drop for Mark {
 fn drop(&mut self) {
 if self.value==1 {panic!("first");}
 else {panic!("second");}
 }
}
struct Pair {first:Mark,second:Mark}
fn probe()->i64 {let mut values=Vec::new(); values.push(Mark {value:1}); values.push(Mark {value:2}); 0}
