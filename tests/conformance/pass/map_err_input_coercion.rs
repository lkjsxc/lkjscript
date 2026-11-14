fn width(text: &str) -> usize { text.len() }
fn failed() -> Result<usize, String> { Err("xx".to_string()) }
pub fn probe() -> i64 {
    failed().map_err(|message| width(&message)).unwrap_err() as i64 + 40
}
