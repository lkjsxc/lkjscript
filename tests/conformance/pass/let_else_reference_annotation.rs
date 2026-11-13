fn good<'a>(value:&'a mut i64){let Some(pointer):Option<&'a mut i64>=Some(&mut *value) else{return};*pointer=42;}pub fn probe()->i64{let mut value=0i64;good(&mut value);value}
