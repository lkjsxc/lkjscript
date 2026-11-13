trait Value { type Item; fn get(&self)->Self::Item; }
struct Number { value:i64 }
impl Value for Number {type Item=i64;fn get(&self)->i64{self.value}}
struct Map<I,F>{input:I,function:F}
impl<I:Value,F:FnMut(I::Item)->i64> Map<I,F>{fn run(&mut self)->i64{(self.function)(self.input.get())}}
pub fn probe()->i64{let mut map=Map{input:Number{value:42},function:|value:bool|if value{42}else{0}};map.run()}
