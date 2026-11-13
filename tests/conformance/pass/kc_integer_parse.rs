pub fn probe() -> i64 {
    assert!("255".parse::<u8>().unwrap() == 255u8);
    assert!("256".parse::<u8>().is_err());
    assert!("-0".parse::<u8>().is_err());
    assert!("65535".parse::<u16>().unwrap() == 65535u16);
    assert!("65536".parse::<u16>().is_err());
    assert!("4294967295".parse::<u32>().unwrap() == 4294967295u32);
    assert!("4294967296".parse::<u32>().is_err());
    assert!("18446744073709551615".parse::<u64>().unwrap() == 18446744073709551615u64);
    assert!("18446744073709551616".parse::<u64>().is_err());
    assert!("+42".parse::<usize>().unwrap() == 42usize);
    assert!("-128".parse::<i8>().unwrap() == -128i8);
    assert!("127".parse::<i8>().unwrap() == 127i8);
    assert!("-129".parse::<i8>().is_err());
    assert!("128".parse::<i8>().is_err());
    assert!("-32768".parse::<i16>().unwrap() == -32768i16);
    assert!("32768".parse::<i16>().is_err());
    assert!("-2147483648".parse::<i32>().unwrap() == -2147483648i32);
    assert!("2147483648".parse::<i32>().is_err());
    assert!("-9223372036854775808".parse::<i64>().unwrap() == -9223372036854775808i64);
    assert!("9223372036854775807".parse::<i64>().unwrap() == 9223372036854775807i64);
    assert!("9223372036854775808".parse::<i64>().is_err());
    assert!("-9223372036854775809".parse::<i64>().is_err());
    assert!("-0".parse::<isize>().unwrap() == 0isize);
    assert!("".parse::<i64>().is_err());
    assert!("+".parse::<i64>().is_err());
    assert!("-".parse::<i64>().is_err());
    assert!(" 42".parse::<i64>().is_err());
    assert!("42 ".parse::<i64>().is_err());
    assert!("4_2".parse::<i64>().is_err());
    assert!("１２".parse::<i64>().is_err());
    assert!("0x2a".parse::<i64>().is_err());
    "000000000000000000000000000042".parse::<i64>().unwrap()
}
