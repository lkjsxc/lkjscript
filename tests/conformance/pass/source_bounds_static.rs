fn choose<'a>(input: Option<&'a str>) -> &'a str {
    match input {
        Option::Some(value) => return value,
        Option::None => {},
    }
    "static"
}
pub fn probe() -> i64 { choose(Option::None).len() as i64 + 36 }
