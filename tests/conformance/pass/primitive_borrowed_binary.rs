fn identity(value: &i64) -> &i64 {
    value
}
fn mask(bits: &u64) -> u64 {
    bits & 0x7ff0000000000000
}
pub fn probe() -> i64 {
    let left = 20_i64;
    let right = 22_i64;
    let float = &1.5_f64 + 0.5_f64;
    let boolean = &true & &false;
    if float != 2.0 || boolean || mask(&0x7ff8000000000000_u64) != 0x7ff0000000000000 {
        return 0;
    }
    identity(&left) + &right
}
