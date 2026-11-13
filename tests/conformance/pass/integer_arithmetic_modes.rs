fn record(hash: &mut u64, value: u64) {
    *hash = (*hash).wrapping_mul(1099511628211u64).wrapping_add(value);
}

fn u8_cases(hash: &mut u64) {
    let values = [0u8, 1u8, 2u8, 127u8, 128u8, 254u8, 255u8];
    for i in 0..values.len() {
        for j in 0..values.len() {
            let left = values[i];
            let right = values[j];
            record(hash, left.wrapping_add(right) as u64);
            match left.checked_add(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_sub(right) as u64);
            match left.checked_sub(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_mul(right) as u64);
            match left.checked_mul(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
        }
    }
}

fn u16_cases(hash: &mut u64) {
    let values = [0u16, 1u16, 2u16, 32767u16, 32768u16, 65534u16, 65535u16];
    for i in 0..values.len() {
        for j in 0..values.len() {
            let left = values[i];
            let right = values[j];
            record(hash, left.wrapping_add(right) as u64);
            match left.checked_add(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_sub(right) as u64);
            match left.checked_sub(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_mul(right) as u64);
            match left.checked_mul(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
        }
    }
}

fn u32_cases(hash: &mut u64) {
    let values = [0u32, 1u32, 2u32, 2147483647u32, 2147483648u32, 4294967294u32, 4294967295u32];
    for i in 0..values.len() {
        for j in 0..values.len() {
            let left = values[i];
            let right = values[j];
            record(hash, left.wrapping_add(right) as u64);
            match left.checked_add(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_sub(right) as u64);
            match left.checked_sub(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_mul(right) as u64);
            match left.checked_mul(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
        }
    }
}

fn u64_cases(hash: &mut u64) {
    let values = [0u64, 1u64, 2u64, 9223372036854775807u64, 9223372036854775808u64, 18446744073709551614u64, 18446744073709551615u64];
    for i in 0..values.len() {
        for j in 0..values.len() {
            let left = values[i];
            let right = values[j];
            record(hash, left.wrapping_add(right) as u64);
            match left.checked_add(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_sub(right) as u64);
            match left.checked_sub(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_mul(right) as u64);
            match left.checked_mul(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
        }
    }
}

fn usize_cases(hash: &mut u64) {
    let values = [0usize, 1usize, 2usize, 9223372036854775807usize, 9223372036854775808usize, 18446744073709551614usize, 18446744073709551615usize];
    for i in 0..values.len() {
        for j in 0..values.len() {
            let left = values[i];
            let right = values[j];
            record(hash, left.wrapping_add(right) as u64);
            match left.checked_add(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_sub(right) as u64);
            match left.checked_sub(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_mul(right) as u64);
            match left.checked_mul(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
        }
    }
}

fn i8_cases(hash: &mut u64) {
    let values = [-128i8, -127i8, -2i8, -1i8, 0i8, 1i8, 2i8, 126i8, 127i8];
    for i in 0..values.len() {
        for j in 0..values.len() {
            let left = values[i];
            let right = values[j];
            record(hash, left.wrapping_add(right) as u64);
            match left.checked_add(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_sub(right) as u64);
            match left.checked_sub(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_mul(right) as u64);
            match left.checked_mul(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
        }
    }
}

fn i16_cases(hash: &mut u64) {
    let values = [-32768i16, -32767i16, -2i16, -1i16, 0i16, 1i16, 2i16, 32766i16, 32767i16];
    for i in 0..values.len() {
        for j in 0..values.len() {
            let left = values[i];
            let right = values[j];
            record(hash, left.wrapping_add(right) as u64);
            match left.checked_add(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_sub(right) as u64);
            match left.checked_sub(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_mul(right) as u64);
            match left.checked_mul(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
        }
    }
}

fn i32_cases(hash: &mut u64) {
    let values = [-2147483648i32, -2147483647i32, -2i32, -1i32, 0i32, 1i32, 2i32, 2147483646i32, 2147483647i32];
    for i in 0..values.len() {
        for j in 0..values.len() {
            let left = values[i];
            let right = values[j];
            record(hash, left.wrapping_add(right) as u64);
            match left.checked_add(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_sub(right) as u64);
            match left.checked_sub(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_mul(right) as u64);
            match left.checked_mul(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
        }
    }
}

fn i64_cases(hash: &mut u64) {
    let values = [-9223372036854775808i64, -9223372036854775807i64, -2i64, -1i64, 0i64, 1i64, 2i64, 9223372036854775806i64, 9223372036854775807i64];
    for i in 0..values.len() {
        for j in 0..values.len() {
            let left = values[i];
            let right = values[j];
            record(hash, left.wrapping_add(right) as u64);
            match left.checked_add(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_sub(right) as u64);
            match left.checked_sub(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_mul(right) as u64);
            match left.checked_mul(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
        }
    }
}

fn isize_cases(hash: &mut u64) {
    let values = [-9223372036854775808isize, -9223372036854775807isize, -2isize, -1isize, 0isize, 1isize, 2isize, 9223372036854775806isize, 9223372036854775807isize];
    for i in 0..values.len() {
        for j in 0..values.len() {
            let left = values[i];
            let right = values[j];
            record(hash, left.wrapping_add(right) as u64);
            match left.checked_add(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_sub(right) as u64);
            match left.checked_sub(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
            record(hash, left.wrapping_mul(right) as u64);
            match left.checked_mul(right) {
                Some(value) => { record(hash, 1); record(hash, value as u64); }
                None => record(hash, 0),
            }
        }
    }
}

pub fn probe() -> i64 {
    let mut hash = 1469598103934665603u64;
    u8_cases(&mut hash);
    u16_cases(&mut hash);
    u32_cases(&mut hash);
    u64_cases(&mut hash);
    usize_cases(&mut hash);
    i8_cases(&mut hash);
    i16_cases(&mut hash);
    i32_cases(&mut hash);
    i64_cases(&mut hash);
    isize_cases(&mut hash);
    (hash & 9223372036854775807u64) as i64
}
