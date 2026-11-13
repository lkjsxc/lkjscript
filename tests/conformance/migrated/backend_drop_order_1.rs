struct Mark { value:i64 }
impl Drop for Mark {
 fn drop(&mut self) {
 if self.value==1 {panic!("first");}
 else {panic!("second");}
 }
}
struct Pair {first:Mark,second:Mark}
fn probe()->i64 {let values=[Mark {value:1}, Mark {value:2}]; 0}
