pub fn probe()->i64 { let mut value=42i64;let reference=&mut value;let make=move||(reference,reference);let pair=make();*pair.0+*pair.1 }
