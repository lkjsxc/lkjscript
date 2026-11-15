// expect: reject E_UNSUPPORTED
// rewrite: Suggest `text.as_str().parse::<u16>()` when parse is available on str but the receiver is String.
pub fn probe() -> u16 { "123".to_string().parse::<u16>().unwrap() }
