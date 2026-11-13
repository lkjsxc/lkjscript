pub fn probe()->i64{let mut value=42i64;let selected=Some(&mut value).map(|input:&mut i64|(input,input)).unwrap();*selected.0+*selected.1}
