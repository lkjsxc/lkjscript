pub fn probe()->i64 { let mut value=42i64; let reference=&mut value; let callable=move || reference; drop(callable()); let result=callable(); *result }
