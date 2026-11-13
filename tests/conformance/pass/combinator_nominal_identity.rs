struct Option{value:i64}
impl Option{fn map<F:FnOnce(i64)->i64>(self,function:F)->i64{function(self.value)+1}}
pub fn probe()->i64{Option{value:40}.map(|x:i64|x+1)}
