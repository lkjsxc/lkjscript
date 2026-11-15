// expect: value 42
use std::json::{Value, parse, stringify};

fn exact(text: &str) {
    let value = parse(text.as_bytes()).unwrap();
    assert_eq!(stringify(&value).unwrap(), text);
}
pub fn probe() -> i64 {
    let numbers = vec![
        "0".to_string(), "-0".to_string(), "1".to_string(), "-1".to_string(), "10".to_string(), "0.0".to_string(), "-0.0".to_string(), "1e0".to_string(), "1E+9".to_string(),
        "1e-1000000".to_string(), "1.23456789012345678901234567890".to_string(),
        "9007199254740993".to_string(), "18446744073709551615".to_string(),
        "18446744073709551616".to_string(), "-9223372036854775809".to_string(),
    ];
    for index in 0usize..numbers.len() { exact(numbers[index].as_str()); }
    exact("null");
    exact("true");
    exact("false");
    exact("\"\"");
    exact("[]");
    exact("{}");
    assert_eq!(parse(b"9007199254740993").unwrap().as_u64(), Some(9007199254740993u64));
    assert_eq!(parse(b"18446744073709551615").unwrap().as_u64(), Some(!0u64));
    assert!(parse(b"18446744073709551616").unwrap().as_u64().is_none());
    assert!(parse(b"1e0").unwrap().as_u64().is_none());
    assert!(parse(b"1.0").unwrap().as_u64().is_none());
    assert!(parse(b"-0").unwrap().as_u64().is_none());
    assert!(Value::Number("01".to_string()).as_u64().is_none());
    assert!(Value::Number("+1".to_string()).as_u64().is_none());
    assert_eq!(Value::from_u64(!0u64).as_u64(), Some(!0u64));
    assert_eq!(Value::from_usize(!0usize).as_usize(), Some(!0usize));
    let value = parse(" \t\r\n{\"text\":\"A\\u0000é𝄞\",\"id\":9007199254740993,\"items\":[null,true,false,{\"empty\":[]}]} \n".as_bytes()).unwrap();
    assert_eq!(value.get("text").unwrap().as_str().unwrap(), "A\0é𝄞");
    assert_eq!(value.get("id").unwrap().as_number().unwrap(), "9007199254740993");
    assert_eq!(value.get("id").unwrap().as_usize(), Some(9007199254740993usize));
    assert_eq!(value.as_object().unwrap().len(), 3usize);
    assert!(value.get("absent").is_none());
    assert!(value.as_str().is_none());
    let items = value.get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 4usize);
    assert_eq!(items[1usize].as_bool(), Some(true));
    assert_eq!(items[2usize].as_bool(), Some(false));
    assert!(items[0usize].as_bool().is_none());
    assert!(items[3usize].get("empty").unwrap().as_array().unwrap().is_empty());
    let text = stringify(&value).unwrap();
    assert_eq!(text, "{\"text\":\"A\\u0000é𝄞\",\"id\":9007199254740993,\"items\":[null,true,false,{\"empty\":[]}]}");
    assert_eq!(stringify(&parse(text.as_bytes()).unwrap()).unwrap(), text);
    42
}
