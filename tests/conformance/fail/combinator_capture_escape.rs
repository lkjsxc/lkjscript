pub fn probe()->i64{let result:&i64;{let short=42i64;let captured=&short;result=Some(0i64).map(move|_:i64|captured).unwrap();}*result}
