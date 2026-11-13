fn invalid<'a, F: FnMut(&'a mut i64) -> i64>(mut function: F, value: &'a mut i64) -> i64 {
    let result = function(value);
    *value = 0;
    result
}

pub fn probe() -> i64 {
    42
}
