fn use_twice<T,F:FnMut()->T>(mut function:F)->T { drop(function()); function() }
pub fn probe()->i64 { let mut value=42i64; let reference=&mut value; let result=use_twice(move || reference); *result }
