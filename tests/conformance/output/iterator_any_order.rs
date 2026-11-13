struct M{out:Stdout,b:u8}
impl Drop for M{fn drop(&mut self){self.out.write(&[self.b]);}}
fn tick(world:&mut World,byte:u8){let mut out=world.stdout();out.write(&[byte]);}
fn make(world:&mut World,byte:u8)->M{tick(world,byte+100u8);M{out:world.stdout(),b:byte}}
struct ReducerSource { values:Vec<M>, out:Stdout, calls:usize, gap:usize }
fn reducer_source(world:&mut World)->ReducerSource {
 tick(world,80u8);let mut values:Vec<M>=Vec::new();
 values.push(make(world,4u8));values.push(make(world,5u8));values.push(make(world,6u8));values.push(make(world,7u8));
 ReducerSource{values,out:world.stdout(),calls:0,gap:0usize}
}
impl Iterator for ReducerSource {
 type Item=M;
 fn next(&mut self)->Option<M>{
  self.out.write(&[90u8]);self.calls=self.calls+1;
  if self.calls==self.gap{None}else{self.values.pop()}
 }
}
impl Drop for ReducerSource{fn drop(&mut self){self.out.write(&[91u8]);}}
fn reducer_receiver<'a>(world:&mut World,iterator:&'a mut ReducerSource)->&'a mut ReducerSource{tick(world,88u8);iterator}
fn reducer_capture(value:&M)->u8{value.b}
pub fn main(mut world:World){let mut iterator=reducer_source(&mut world);let answer=reducer_receiver(&mut world,&mut iterator).any({tick(&mut world,81u8);let capture=make(&mut world,62u8);let mut out=world.stdout();move|value|{out.write(&[reducer_capture(&capture)]);value.b==5u8}});tick(&mut world,if answer{200u8}else{201u8});let mut round=0usize;while round<2usize{match iterator.next(){Some(value)=>{tick(&mut world,value.b);drop(value);},None=>tick(&mut world,202u8)}round=round+1;}drop(iterator);tick(&mut world,83u8);tick(&mut world,99u8);}
