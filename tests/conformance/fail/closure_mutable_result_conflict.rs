pub fn probe()->i64 {let mut first=20i64;let mut second=22i64;let a=&mut first;let b=&mut second;let make=move||(a,b);let pair=make();first=0;*pair.0+*pair.1}
