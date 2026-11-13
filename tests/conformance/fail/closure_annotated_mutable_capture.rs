pub fn probe()->i64 { let mut value=42i64; let reference=&mut value; let mut callable=move || -> &mut i64 { reference }; let result=callable(); *result }
