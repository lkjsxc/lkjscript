fn argument_selected()->i64{
 let value=42i64;let result:&i64;
 {let short=1i64;let capture=&short;result=Some(&value).map(move|input:&i64|{let _=*capture;input}).unwrap();}
 *result
}
fn capture_selected()->i64{
 let value=42i64;let capture=&value;let result:&i64;
 {let short=1i64;result=Some(&short).map(move|_:&i64|capture).unwrap();}
 *result
}
fn mutable_pair()->i64{
 let mut left=20i64;let mut right=22i64;
 let selected=Some((&mut left,&mut right)).map(|pair:(&mut i64,&mut i64)|(pair.1,pair.0)).unwrap();
 *selected.0=*selected.0+1;*selected.1=*selected.1-1;*selected.0+*selected.1
}
pub fn probe()->i64{
 assert!(argument_selected()==42&&capture_selected()==42&&mutable_pair()==42);
 let value=42i64;let result:Result<i64,&i64>=Err(&value);
 let selected=result.map_err(|input:&i64|input).unwrap_err();assert!(*selected==42);
 let missing:Option<&i64>=None;let selected=missing.or_else(||Some(&value)).unwrap();assert!(*selected==42);
 let absent:Option<&i64>=None;assert!(*absent.unwrap_or_else(||&value)==42);
 let failed:Result<&i64,i64>=Err(0);
 let recovered:Result<&i64,i64>=failed.or_else(|_:i64| -> Result<&i64,i64>{Ok(&value)});assert!(*recovered.unwrap()==42);
 let failed:Result<&i64,i64>=Err(0);assert!(*failed.unwrap_or_else(|_:i64|&value)==42);
 let values=[1u8,19,23,7];let all:&[u8]=&values;
 let part=Some(all).map(|bytes:&[u8]|&bytes[1..3]).unwrap();part[0] as i64+part[1] as i64
}
