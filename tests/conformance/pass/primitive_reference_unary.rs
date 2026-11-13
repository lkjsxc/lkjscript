pub fn probe() -> i64 {
    let flag = false;
    let value = 7i64;
    let bits = 1u8;
    let flag_ref = &flag;
    let value_ref = &value;
    let bits_ref = &bits;
    if !flag_ref && -value_ref == -7i64 && !bits_ref == 254u8 { 42 } else { 0 }
}
