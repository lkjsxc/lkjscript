#[path = "../bridge/library.rs"]
mod bootstrap_library;
// Existing Rust library behavior and native execution use the same probes.
#[path = "support/native.rs"]
mod native;
use lkjscript::{k, source};
use std::process::Command;

#[path = "../library/bytes.rs"]
#[allow(dead_code)]
mod bytes;
use bytes as rust_bytes;
#[path = "../library/ordered_map.rs"]
#[allow(dead_code)]
mod rust_map;
#[path = "../library/prelude.rs"]
#[allow(dead_code)]
pub mod rust_prelude;
pub use rust_prelude::{Option, Result};
mod prelude {
    pub use super::rust_prelude::Option;
}
#[path = "../library/integer.rs"]
#[allow(dead_code)]
mod integer;
#[path = "../library/iter.rs"]
#[allow(dead_code)]
mod iter;

fn compile(text: &str, _library: &str) -> k::CheckedProgram {
    let text = format!("use std::bytes::*; use std::collections::BTreeMap;\n{text}");
    let mut files = bootstrap_library::standard_library();
    files.push(source::SourceInput {
        path: "probe.rs".to_string(),
        text,
    });
    let bundle = source::SourceBundle {
        root: "probe.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    };
    native::check(&bundle)
}

fn run(checked: &k::CheckedProgram, entry: &str, expected: i64) {
    native::assert_values(checked, &[(entry, expected)]);
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
    run(&compile(source, ""), "prelude_probe", expected);
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
    rust_bytes::append_bytes(&mut bytes, &[67, 240, 159, 152, 128]);
    assert!(rust_bytes::starts_with(&bytes, &[65, 66, 67]));
    assert!(rust_bytes::valid_utf8(&bytes));
    assert!(!rust_bytes::valid_utf8(&[237, 160, 128]));
    let expected = rust_bytes::find_byte(&bytes, 67).unwrap() as i64 + bytes.len() as i64;
    run(
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
            let observed = { let found = map.get(&3).unwrap(); **found };
            { let found = map.get_mut(&4).unwrap(); **found = 44; }
            let changed = { let found = map.get(&4).unwrap(); **found };
            let removed = map.remove(&1).unwrap();
            *previous + *removed + map.len() as i64 + observed + changed
        }
    "#;
    let mut map = rust_map::BTreeMap::new();
    assert!(map.insert(4_i64, Box::new(40_i64)).is_none());
    assert!(map.insert(1, Box::new(10)).is_none());
    assert!(map.insert(3, Box::new(30)).is_none());
    let previous = map.insert(3, Box::new(33)).unwrap();
    assert!(map.contains_key(&1));
    assert!(!map.contains_key(&2));
    let observed = {
        let found = map.get(&3).unwrap();
        **found
    };
    {
        let found = map.get_mut(&4).unwrap();
        **found = 44;
    }
    let changed = {
        let found = map.get(&4).unwrap();
        **found
    };
    let removed = map.remove(&1).unwrap();
    let expected = *previous + *removed + map.len() as i64 + observed + changed;
    run(
        &compile(source, include_str!("../library/ordered_map.rs")),
        "map_probe",
        expected,
    );
}

#[path = "../library/string.rs"]
#[allow(dead_code)]
mod rust_string;
#[path = "../library/str.rs"]
#[allow(dead_code)]
mod str;
mod string {
    pub use super::rust_string::String;
}

#[test]
fn strings_validate_and_project_borrowed_bytes() {
    let source = r#"
        fn string_probe() -> i64 {
            let input: Vec<u8> = vec![65, 240, 159, 152, 128];
            let mut text = String::from_utf8(input).unwrap();
            assert_eq!(text.len(), 5);
            let first = {
                let view: &[u8] = text.as_bytes();
                view[0] as i64
            };
            text.clear();
            assert!(text.is_empty());
            let invalid: Vec<u8> = vec![255];
            let failure = String::from_utf8(invalid).unwrap_err();
            let recovered = failure.into_bytes();
            first + recovered[0] as i64
        }
    "#;
    let mut text = rust_string::String::from_utf8(vec![65, 240, 159, 152, 128]).unwrap();
    assert_eq!(text.len(), 5);
    let first = {
        let view = text.as_bytes();
        i64::from(view[0])
    };
    text.clear();
    assert!(text.is_empty());
    let recovered = rust_string::String::from_utf8(vec![255])
        .unwrap_err()
        .into_bytes();
    let expected = first + i64::from(recovered[0]);
    let library = format!(
        "{}\n{}",
        include_str!("../library/bytes.rs"),
        include_str!("../library/string.rs")
    );
    run(&compile(source, &library), "string_probe", expected);
}

#[test]
fn complete_source_library_builds_with_rustc() {
    let directory = std::path::Path::new(".artifacts");
    std::fs::create_dir_all(directory).unwrap();
    let output = directory.join(format!("source-library-{}.rlib", std::process::id()));
    let result = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2024",
            "-C",
            "overflow-checks=yes",
        ])
        .arg("library/lib.rs")
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        std::string::String::from_utf8_lossy(&result.stderr)
    );
    std::fs::remove_file(output).unwrap();
}

#[test]
fn byte_vector_keys_use_source_lexicographic_order() {
    let source = r#"
        fn byte_keys_probe() -> i64 {
            let mut map: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
            map.insert(vec![3, 2], 32);
            map.insert(vec![1], 10);
            map.insert(vec![3, 1], 31);
            map.insert(vec![3, 2, 0], 320);
            let key: Vec<u8> = vec![3, 2];
            let value = { let found = map.get(&key).unwrap(); *found };
            assert_eq!(key.len(), 2);
            let before: Vec<u8> = vec![3, 1];
            let removed = map.remove(&before).unwrap();
            value as i64 + removed as i64 + map.len() as i64
        }
    "#;
    let mut map = rust_map::BTreeMap::new();
    map.insert(vec![3_u8, 2], 32_usize);
    map.insert(vec![1], 10);
    map.insert(vec![3, 1], 31);
    map.insert(vec![3, 2, 0], 320);
    let key = vec![3_u8, 2];
    let value = {
        let found = map.get(&key).unwrap();
        *found
    };
    let removed = map.remove(&vec![3_u8, 1]).unwrap();
    let expected = value as i64 + removed as i64 + map.len() as i64;
    let library = format!(
        "{}\n{}",
        include_str!("../library/ordered_map.rs"),
        include_str!("../library/sequence.rs")
    );
    run(&compile(source, &library), "byte_keys_probe", expected);
}
