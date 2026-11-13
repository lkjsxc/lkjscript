pub fn probe() -> i64 {
    let mut code = 0u16;
    while code < 256u16 {
        let byte = code as u8;
        let digit = code >= 48u16 && code <= 57u16;
        let upper = code >= 65u16 && code <= 90u16;
        let lower = code >= 97u16 && code <= 122u16;
        assert_eq!(byte.is_ascii(), code < 128u16);
        assert_eq!(byte.is_ascii_digit(), digit);
        assert_eq!(byte.is_ascii_uppercase(), upper);
        assert_eq!(byte.is_ascii_lowercase(), lower);
        assert_eq!(byte.is_ascii_alphabetic(), upper || lower);
        assert_eq!(byte.is_ascii_alphanumeric(), upper || lower || digit);
        assert_eq!(
            byte.is_ascii_hexdigit(),
            digit || (code >= 65u16 && code <= 70u16) || (code >= 97u16 && code <= 102u16)
        );
        assert_eq!(byte.is_ascii_control(), code < 32u16 || code == 127u16);
        assert_eq!(
            byte.is_ascii_whitespace(),
            code == 9u16 || code == 10u16 || code == 12u16 || code == 13u16 || code == 32u16
        );
        assert_eq!(byte.is_ascii_graphic(), code >= 33u16 && code <= 126u16);
        assert_eq!(
            byte.is_ascii_punctuation(),
            code >= 33u16 && code <= 126u16 && !(upper || lower || digit)
        );
        assert_eq!(
            byte.to_ascii_lowercase(),
            if upper { byte + 32u8 } else { byte }
        );
        assert_eq!(
            byte.to_ascii_uppercase(),
            if lower { byte - 32u8 } else { byte }
        );
        assert!(byte.eq_ignore_ascii_case(&byte.to_ascii_uppercase()));
        code += 1u16;
    }
    assert_eq!((-128i8).min(127i8), -128i8);
    assert_eq!((-128i8).max(127i8), 127i8);
    assert_eq!(65535u16.min(0u16), 0u16);
    assert_eq!(65535u16.max(0u16), 65535u16);
    assert_eq!(5usize.min(5usize), 5usize);
    assert_eq!(5usize.max(5usize), 5usize);
    assert_eq!('水'.min('A'), 'A');
    assert!(!false.min(true));
    42
}
