fn invalid<'a, B, F: FnMut(&'a mut i64) -> B>(mut function: F, value: &'a mut i64) -> B {
    let result = function(value);
    *value = 0;
    result
}

pub fn probe() -> i64 {
    42
}
