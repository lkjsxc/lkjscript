fn parse(b: u8) -> Result<i64, i64> { if b >= 48 && b <= 57 { Ok((b - 48) as i64) } else { Err(1) } }
fn two(a: u8, b: u8) -> Result<i64, i64> { let x = parse(a)?; let y = parse(b)?; Ok(x * 10 + y) }
pub fn probe() -> i64 { match two(52, 50) { Ok(v) => v, Err(e) => e } }
