pub fn probe()->i64{let owned=Box::new(42i64);let once=move|_:i64|owned;let result=Some(1i64).map(once);let again=once(0i64);*result.unwrap()+*again}
