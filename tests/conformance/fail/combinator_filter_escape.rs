pub fn probe()->i64{let mut escaped:Option<&i64>=None;let value=Some(42i64);let result=value.filter(|input:&i64|{escaped=Some(input);true});*escaped.unwrap()+result.unwrap()}
