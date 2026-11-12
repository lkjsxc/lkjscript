pub fn probe() -> i64 { let x: u8 = 250; let y = x.wrapping_add(10); let z = x.checked_add(10); let w = match z { Some(_) => 1, None => 0 }; y as i64 + w + 38 }
