//! Existing Rust library behavior and Node execution are compared on the same probes.
use lkjscript::{check, elaborate, syntax, wasm};
use std::io::Write;
use std::process::{Command, Stdio};

#[path = "../library/bytes.rs"]
#[allow(dead_code)]
mod rust_bytes;
#[path = "../library/ordered_map.rs"]
#[allow(dead_code)]
mod rust_map;
#[path = "../library/prelude.rs"]
#[allow(dead_code)]
mod rust_prelude;

fn compile(source: &str, library: &str) -> Vec<u8> {
    let all = format!(
        "{}\n{library}\n{source}",
        include_str!("../library/prelude.rs")
    );
    let parsed = syntax::parse(&all).unwrap();
    let program = elaborate::elaborate(&parsed).unwrap();
    wasm::emit(&check::check(program).unwrap()).unwrap()
}
fn run_node(bytes: &[u8], entry: &str, expected: i64) {
    let code = format!(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));\
         process.stdin.on('end',async()=>{{try{{\
         const {{instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),\
         {{lkjscript:{{trap(c,l){{throw Error('trap '+c+':'+l)}}}}}});\
         const value=instance.exports['{entry}']();\
         if(value!=={expected}n)throw Error('expected {expected}, got '+value);\
         }}catch(e){{console.error(e);process.exitCode=1}}}});"
    );
    let mut child = Command::new("node")
        .arg("-e")
        .arg(code)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn option_and_result_own_affine_payloads() {
    let source = r#"
        fn prelude_probe() -> i64 {
            let mut present: Option<Box<i64>> = Option::Some(Box::new(17));
            assert!(present.is_some());
            let held = present.take();
            assert!(present.is_none());
            let value = held.unwrap();
            let absent: Option<i64> = Option::None;
            let fallback = absent.unwrap_or(5);
            let result: Result<i64, i64> = Result::Ok(*value + fallback);
            assert!(result.is_ok());
            result.unwrap()
        }
    "#;
    let mut present = rust_prelude::Option::Some(Box::new(17_i64));
    assert!(present.is_some());
    let held = present.take();
    assert!(present.is_none());
    let value = held.unwrap();
    let absent: rust_prelude::Option<i64> = rust_prelude::Option::None;
    let expected = rust_prelude::Result::<i64, i64>::Ok(*value + absent.unwrap_or(5)).unwrap();
    run_node(&compile(source, ""), "prelude_probe", expected);
}

#[test]
fn byte_readers_match_rust() {
    let source = r#"
        fn bytes_probe() -> i64 {
            let mut bytes: Vec<u8> = vec![65, 66];
            let tail: Vec<u8> = vec![67, 240, 159, 152, 128];
            append_bytes(&mut bytes, &tail);
            let prefix: Vec<u8> = vec![65, 66, 67];
            assert!(starts_with(&bytes, &prefix));
            assert!(valid_utf8(&bytes));
            let invalid: Vec<u8> = vec![237, 160, 128];
            assert!(!valid_utf8(&invalid));
            find_byte(&bytes, 67).unwrap() as i64 + bytes.len() as i64
        }
    "#;
    let mut bytes = vec![65_u8, 66];
    rust_bytes::append_bytes(&mut bytes, &vec![67, 240, 159, 152, 128]);
    assert!(rust_bytes::starts_with(&bytes, &vec![65, 66, 67]));
    assert!(rust_bytes::valid_utf8(&bytes));
    assert!(!rust_bytes::valid_utf8(&vec![237, 160, 128]));
    let expected = rust_bytes::find_byte(&bytes, 67).unwrap() as i64 + bytes.len() as i64;
    run_node(
        &compile(source, include_str!("../library/bytes.rs")),
        "bytes_probe",
        expected,
    );
}

#[test]
fn ordered_map_inserts_replaces_and_removes_affine_values() {
    let source = r#"
        fn map_probe() -> i64 {
            let mut map: BTreeMap<i64, Box<i64>> = BTreeMap::new();
            assert!(map.insert(4, Box::new(40)).is_none());
            assert!(map.insert(1, Box::new(10)).is_none());
            assert!(map.insert(3, Box::new(30)).is_none());
            let previous = map.insert(3, Box::new(33)).unwrap();
            assert!(map.contains_key(&1));
            assert!(!map.contains_key(&2));
            let removed = map.remove(&1).unwrap();
            *previous + *removed + map.len() as i64
        }
    "#;
    let mut map = rust_map::BTreeMap::new();
    assert!(map.insert(4_i64, Box::new(40_i64)).is_none());
    assert!(map.insert(1, Box::new(10)).is_none());
    assert!(map.insert(3, Box::new(30)).is_none());
    let previous = map.insert(3, Box::new(33)).unwrap();
    assert!(map.contains_key(&1));
    assert!(!map.contains_key(&2));
    let removed = map.remove(&1).unwrap();
    let expected = *previous + *removed + map.len() as i64;
    run_node(
        &compile(source, include_str!("../library/ordered_map.rs")),
        "map_probe",
        expected,
    );
}
