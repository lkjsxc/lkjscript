pub fn probe()->i64{let mut value=42i64;let selected=Some(&mut value).map(|input:&mut i64|input).unwrap();value=1;*selected}
