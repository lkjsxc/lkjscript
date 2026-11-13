struct Mark { value:i64 }
impl Drop for Mark {
 fn drop(&mut self) {
 if self.value==1 {panic!("first");}
 else {panic!("second");}
 }
}
struct Pair {first:Mark,second:Mark}
fn probe()->i64 {let pair=Pair { first:Mark {value:1}, second:Mark {value:2} }; 0}
