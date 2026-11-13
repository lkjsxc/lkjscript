fn some<I:Iterator,F:FnMut(I::Item)->bool>(input:&mut I,predicate:F)->bool{input.any(predicate)}
pub fn probe()->i64{
 let values=vec![1i64,2,3,4];let mut input=values.iter();let mut calls=0i64;
 assert!(some(&mut input,|value|{calls=calls+1;*value==2}));assert!(calls==2);
 assert!(*input.next().unwrap()==3);
 assert!(!input.all(|value|{calls=calls+1;*value<4}));assert!(calls==3);
 assert!(input.next().is_none());
 let empty:Vec<i64>=Vec::new();
 let mut absent=empty.iter();assert!(!absent.any(|_|{assert!(false);true}));
 assert!(absent.all(|_|{assert!(false);false}));
 let mut every=values.iter();assert!(every.all(|value|*value<5));assert!(every.next().is_none());
 42
}
