pub fn probe()->i64{
 let mut calls=0i64;
 let value=Some(Box::new(42i64));
 let selected=value.filter(|value:&Box<i64>|{calls=calls+1;**value==42});
 assert!(calls==1);calls=0;
 let empty:Option<Box<i64>>=None;
 let unused=empty.filter(|value:&Box<i64>|{calls=calls+1;**value==42});
 assert!(unused.is_none()&&calls==0);
 let mut number=42i64;
 let chosen=Some(&mut number).filter(|value:&&mut i64|**value==42).unwrap();
 *chosen=40;drop(chosen);number=number+2;
 assert!(number==42);*selected.unwrap()
}
