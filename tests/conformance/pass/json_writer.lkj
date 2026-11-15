// expect: value 42
use std::json::{Value, parse, stringify};

pub fn probe() -> i64 {
    let invalid = vec!["".to_string(), "+1".to_string(), "01".to_string(), "-01".to_string(), " 1".to_string(), "1 ".to_string(), "-".to_string(), "1.".to_string(), "1e+".to_string(), "NaN".to_string(), "Infinity".to_string(), "1,true".to_string()];
    for index in 0usize..invalid.len() {
        let value = Value::Number(invalid[index].to_string());
        assert!(stringify(&value).is_err());
    }
    let duplicate = Value::Object(vec![
        ("key".to_string(), Value::Null),
        ("key".to_string(), Value::Bool(true)),
    ]);
    assert_eq!(stringify(&duplicate).unwrap_err().message, "duplicate object key");
    let nested_bad = Value::Array(vec![
        Value::Bool(true), Value::Number("1e+".to_string()),
    ]);
    assert_eq!(stringify(&nested_bad).unwrap_err().offset, 9usize);
    let object = Value::Object(vec![
        ("quote\"\\\n".to_string(), Value::String("text\0é😀".to_string())),
        ("id".to_string(), Value::from_u64(9007199254740993u64)),
        ("empty".to_string(), Value::Array(Vec::new())),
    ]);
    let encoded = stringify(&object).unwrap();
    assert_eq!(encoded, "{\"quote\\\"\\\\\\n\":\"text\\u0000é😀\",\"id\":9007199254740993,\"empty\":[]}");
    assert_eq!(stringify(&parse(encoded.as_bytes()).unwrap()).unwrap(), encoded);
    nested_roundtrip(32usize);
    wide_roundtrip(32usize);
    42
}
fn nested_roundtrip(depth: usize) {
    let mut nested = String::new();
    for index in 0usize..depth {
        if index % 2usize == 0usize { nested.push('['); }
        else { nested.push_str("{\"key\":"); }
    }
    nested.push_str("null");
    let mut index = depth;
    while index > 0usize {
        index -= 1usize;
        if index % 2usize == 0usize { nested.push(']'); } else { nested.push('}'); }
    }
    assert_eq!(stringify(&parse(nested.as_bytes()).unwrap()).unwrap(), nested);
}
fn wide_roundtrip(count: usize) {
    let mut members = Vec::new();
    for index in 0usize..count {
        members.push((format!("key{index}"), Value::Array(vec![
            Value::from_usize(index), Value::Bool(index % 2usize == 0usize),
        ])));
    }
    let wide = stringify(&Value::Object(members)).unwrap();
    assert_eq!(stringify(&parse(wide.as_bytes()).unwrap()).unwrap(), wide);
}

// Optional deeper stress, already covered natively. The independent interpreter
// is intentionally much slower here; this workload is not a parser nesting cap.
pub fn deep_roundtrip() -> i64 {
    nested_roundtrip(257usize);
    wide_roundtrip(100usize);
    42
}
