struct Pulse{step:u8}
impl Iterator for Pulse{type Item=i64;fn next(&mut self)->Option<i64>{self.step=self.step+1u8;match self.step{1=>Some(1),2=>None,3=>Some(3),4=>Some(4),_=>None}}}
pub fn probe()->i64{
 let mut input=Pulse{step:0};assert!(!input.any(|value|value==9));assert!(input.step==2);
 assert!(!input.all(|value|value<4));assert!(input.step==4);
 assert!(input.all(|_|false));assert!(input.step==5);
 let mut again=Pulse{step:0};assert!(again.all(|value|value>0));assert!(again.step==2);
 assert!(again.next().unwrap()==3);42
}
