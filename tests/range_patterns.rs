//! Integer range-pattern semantics through the ordinary compiler and Rust oracle.
use lkjscript::ast::Diagnostic;
use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, k};
use std::io::Write;
use std::process::{Command, Stdio};

const COMMON_SOURCE: &str = r#"
pub fn u8_direct(value: u8) -> i64 {
    match value { ..32 => 1, 32..=126 => 2, 127.. => 3 }
}
fn u8_reference(value: &u8) -> i64 {
    match value { ..=31 => 1, 32..127 => 2, 127.. => 3 }
}
pub fn u8_borrowed(value: u8) -> i64 { u8_reference(&value) }
pub fn u8_radices(value: u8) -> i64 {
    match value { 0x00u8..0o40u8 => 1, 0b0010_0000u8..=0x7eu8 => 2, 0o177u8.. => 3 }
}
pub fn u8_bytes(value: u8) -> i64 {
    match value { b'\0'..=b' ' => 1, b'!'..=b'~' => 2, 0x7f..=0xff => 3 }
}
pub fn u8_overlap(value: u8) -> i64 {
    match value { 0..=100 => 1, 50..=200 => 2, 201.. => 3 }
}
pub fn u8_unordered(value: u8) -> i64 {
    match value { 200..=255 => 3, ..64 => 1, 64..200 => 2 }
}
pub fn i8_direct(value: i8) -> i64 {
    match value { ..=-65i8 => 1, -64i8..0 => 2, 0 => 3, 1..=63 => 4, 64.. => 5 }
}
fn i64_reference(value: &i64) -> i64 {
    match value {
        -9223372036854775808i64..=-1i64 => 11,
        0i64..9223372036854775807i64 => 12,
        9223372036854775807i64 => 13
    }
}
pub fn i64_borrowed(value: i64) -> i64 { i64_reference(&value) }
fn u64_reference(value: &u64) -> i64 {
    match value {
        0u64..9223372036854775808u64 => 21,
        9223372036854775808u64..18446744073709551615u64 => 22,
        18446744073709551615u64 => 23
    }
}
pub fn u64_borrowed(value: u64) -> i64 { u64_reference(&value) }
pub fn signed_literals(value: i64) -> i64 {
    match value { -9223372036854775808i64 => 31, -1i64 => 32, _ => 33 }
}
pub fn max_suffix(value: u64) -> i64 {
    match value { 0xffff_ffff_ffff_ffffu64.. => 41, ..0xffff_ffff_ffff_ffffu64 => 42 }
}
pub fn negative_singleton(value: i8) -> i64 {
    match value { -128i8..=-128i8 => 51, -127i8.. => 52 }
}
pub fn wildcard_first(value: i64) -> i64 { match value { _ => 1, -1 => 2 } }
struct Counted<'a> { count: &'a mut i64, payload: Box<i64> }
impl<'a> Drop for Counted<'a> {
    fn drop(&mut self) { *self.count = *self.count + 1; }
}
enum Choice<'a> { Empty, Value(Counted<'a>) }
fn select_owned(value: Choice<'_>) -> i64 {
    match value {
        bound => { drop(bound); 42 },
        Choice::Value(other) => { drop(other); 1000 },
        Choice::Empty => 0
    }
}
pub fn default_owned(value: i64) -> i64 {
    let mut count: i64 = 0;
    let selected = select_owned(Choice::Value(Counted { count: &mut count, payload: Box::new(value) }));
    assert_eq!(count, 1);
    selected + value
}
pub fn inferred(value: i64) -> i64 {
    let first = match 255 { 0u8..255 => 1i64, 255 => 2 };
    let second = match -128 { -128i8..=-1 => 3i64, 0.. => 4 };
    first + second + value
}
pub fn inferred_borrowed(value: i64) -> i64 {
    let first = match &7 { 0u8..=9u8 => 1i64, _ => 0 };
    first + value
}
fn mutable_reference(value: &mut u8) -> i64 {
    let selected = match value { ..=31 => 1i64, 32.. => 2 };
    selected + *value as i64
}
pub fn mutable_byte(value: u8) -> i64 {
    let mut local = value;
    mutable_reference(&mut local)
}
"#;

const ORACLE_DRIVER: &str = r#"
fn main() {
    for raw in 0..=255u16 {
        let value = raw as u8;
        println!("u8_direct {} {}", value, u8_direct(value));
        println!("u8_borrowed {} {}", value, u8_borrowed(value));
        println!("u8_radices {} {}", value, u8_radices(value));
        println!("u8_bytes {} {}", value, u8_bytes(value));
        println!("u8_overlap {} {}", value, u8_overlap(value));
        println!("u8_unordered {} {}", value, u8_unordered(value));
        println!("mutable_byte {} {}", value, mutable_byte(value));
        let signed = (raw as i16 - 128) as i8;
        println!("i8_direct {} {}", signed, i8_direct(signed));
        println!("negative_singleton {} {}", signed, negative_singleton(signed));
    }
    for value in [i64::MIN, i64::MIN + 1, -65, -1, 0, 1, i64::MAX - 1, i64::MAX] {
        println!("i64_borrowed {} {}", value, i64_borrowed(value));
        println!("signed_literals {} {}", value, signed_literals(value));
        println!("wildcard_first {} {}", value, wildcard_first(value));
    }
    for value in [-5i64, 0, 42] {
        println!("default_owned {} {}", value, default_owned(value));
        println!("inferred {} {}", value, inferred(value));
        println!("inferred_borrowed {} {}", value, inferred_borrowed(value));
    }
    for value in [0u64, 1, (1u64 << 63) - 1, 1u64 << 63, (1u64 << 63) + 1, u64::MAX - 1, u64::MAX] {
        println!("u64_borrowed {} {}", value, u64_borrowed(value));
        println!("max_suffix {} {}", value, max_suffix(value));
    }
}
"#;

fn bundle(text: &str) -> SourceBundle {
    SourceBundle {
        root: "range_probe.rs".to_string(),
        library: None,
        files: vec![SourceInput {
            path: "range_probe.rs".to_string(),
            text: text.to_string(),
        }],
    }
}

fn checked(text: &str) -> Result<k::CheckedProgram, Diagnostic> {
    compiler::check_bundle(&bundle(text))
}

fn rejected(text: &str, code: &str, reason: &str) {
    match checked(text) {
        Ok(_) => panic!("invalid pattern was accepted: {text}"),
        Err(error) => {
            assert_eq!(error.code, code, "{text}: {error:?}");
            assert!(error.message.contains(reason), "{text}: {error:?}");
        }
    }
}

fn run_node(bytes: &[u8], observations: &str) {
    let script = format!(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{{try{{const {{instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),{{lkjscript:{{trap(c,l){{throw Error('trap '+c+':'+l)}}}}}});{observations}}}catch(e){{console.error(e);process.exitCode=1}}}})"
    );
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node is required for range-pattern execution");
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn exhaustive_integer_domains_and_boundaries_match_the_same_rust_source() {
    let directory =
        std::path::Path::new(".artifacts").join(format!("range-patterns-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("oracle.rs");
    let binary = directory.join("oracle");
    std::fs::write(&source, format!("{COMMON_SOURCE}\n{ORACLE_DRIVER}")).unwrap();
    let compiled = Command::new("rustc")
        .args([
            "--edition=2024",
            "--cap-lints=allow",
            "-C",
            "overflow-checks=yes",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("rustc is required for the common-subset oracle");
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let oracle = Command::new(&binary).output().unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let expected = String::from_utf8(oracle.stdout).unwrap();
    let mut rows = String::new();
    let mut count = 0;
    for line in expected.lines() {
        let columns: Vec<_> = line.split_whitespace().collect();
        assert_eq!(columns.len(), 3, "unexpected oracle line: {line}");
        rows.push_str(&format!(
            "['{}','{}','{}'],",
            columns[0], columns[1], columns[2]
        ));
        count += 1;
    }
    assert_eq!(count, 256 * 9 + 8 * 3 + 7 * 2 + 3 * 3);
    let bytes =
        compiler::compile(&bundle(COMMON_SOURCE)).unwrap_or_else(|error| panic!("{error:?}"));
    run_node(
        &bytes,
        &format!(
            "for(const [name,input,expected]of[{rows}]){{const result=instance.exports['fn:'+name](BigInt(input));if(result!==BigInt(expected))throw Error(name+'('+input+'): expected '+expected+', got '+result);}}"
        ),
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_single_gap_is_not_reported_as_exhaustive() {
    rejected(
        "fn bad(value:u8)->i64{match value{0..128=>0,129..=255=>1}}",
        "E_TYPE",
        "exhaustive",
    );
    rejected(
        "fn bad(value:i8)->i64{match value{-128..0=>0,1..=127=>1}}",
        "E_TYPE",
        "exhaustive",
    );
    rejected(
        "fn bad(value:u64)->i64{match value{0..18446744073709551615u64=>0}}",
        "E_TYPE",
        "exhaustive",
    );
}

#[test]
fn empty_and_reversed_ranges_are_rejected_before_coverage() {
    for source in [
        "fn bad(value:u8)->i64{match value{4..4=>1,_=>0}}",
        "fn bad(value:u8)->i64{match value{..0=>1,_=>0}}",
        "fn bad(value:i8)->i64{match value{..-128i8=>1,_=>0}}",
        "fn bad(value:u64)->i64{match value{18446744073709551615u64..0u64=>1,_=>0}}",
        "fn bad(value:u8)->i64{match value{5..=4=>1,_=>0}}",
        "fn bad(value:i8)->i64{match value{-2..=-3=>1,_=>0}}",
        "fn bad(value:i8)->i64{match value{-0..0=>1,_=>0}}",
    ] {
        rejected(source, "E_TYPE", "range");
    }
}

#[test]
fn range_endpoints_obey_the_scrutinees_integer_representation() {
    for source in [
        "fn bad(value:u8)->i64{match value{0..=256=>1,_=>0}}",
        "fn bad(value:i8)->i64{match value{-129..=0=>1,_=>0}}",
        "fn bad(value:i8)->i64{match value{0..128=>1,_=>0}}",
        "fn bad(value:u64)->i64{match value{0..=18446744073709551616u64=>1,_=>0}}",
        "fn bad(value:i64)->i64{match value{-9223372036854775809i64..0=>1,_=>0}}",
    ] {
        rejected(source, "E_TYPE", "literal");
    }
    rejected(
        "fn bad(value:u8)->i64{match value{-1..0=>1,_=>0}}",
        "E_TYPE",
        "negated",
    );
}

#[test]
fn range_endpoint_suffixes_do_not_silently_change_the_matched_type() {
    rejected(
        "fn bad(value:u8)->i64{match value{0u16..=1u16=>1,_=>0}}",
        "E_TYPE",
        "type",
    );
    rejected(
        "fn bad(value:i8)->i64{match value{-1i16..=1i16=>1,_=>0}}",
        "E_TYPE",
        "type",
    );
    rejected(
        "fn bad(value:u64)->i64{match value{b'A'..=b'Z'=>1,_=>0}}",
        "E_TYPE",
        "type",
    );
}

#[test]
fn noninteger_range_patterns_are_rejected_by_type() {
    rejected(
        "fn bad(value:bool)->i64{match value{false..=true=>1,_=>0}}",
        "E_TYPE",
        "integer",
    );
    rejected(
        "fn bad(value:f64)->i64{match value{0.0..=1.0=>1,_=>0}}",
        "E_TYPE",
        "integer",
    );
}

#[test]
fn endpoint_names_are_explicitly_unsupported_instead_of_runtime_values() {
    rejected(
        "fn bad(value:u8,limit:u8)->i64{match value{0..limit=>1,_=>0}}",
        "E_UNSUPPORTED",
        "literal",
    );
    rejected(
        "const LIMIT:u8=10;fn bad(value:u8)->i64{match value{0..=LIMIT=>1,_=>0}}",
        "E_UNSUPPORTED",
        "literal",
    );
}

#[test]
fn incomplete_inclusive_ranges_have_a_named_syntax_error() {
    rejected(
        "fn bad(value:u8)->i64{match value{0..= =>1,_=>0}}",
        "E_PARSE",
        "inclusive range pattern needs an end",
    );
    rejected(
        "fn bad(value:u8)->i64{match value{..= =>1,_=>0}}",
        "E_PARSE",
        "inclusive range pattern needs an end",
    );
}

#[test]
fn a_wildcard_does_not_hide_invalid_later_bounds_or_bodies() {
    rejected(
        "fn bad(value:u8)->i64{match value{_=>0,0..=256=>1}}",
        "E_TYPE",
        "literal",
    );
    rejected(
        "fn bad(value:u8)->i64{match value{_=>0,4..4=>1}}",
        "E_TYPE",
        "range",
    );
    rejected(
        "fn bad<T>(value:T)->i64{match 0i64{_=>1,1=>true}}",
        "E_TYPE",
        "type",
    );
}

#[test]
fn unused_generic_bodies_check_range_endpoints_and_dead_matches() {
    rejected(
        "fn bad<T>(value:T)->i64{match 0u8{0..=256=>1,_=>0}}",
        "E_TYPE",
        "literal",
    );
    rejected(
        "fn bad<T>(value:T)->i64{return 0;match 0u8{0..=256=>1,_=>0}}",
        "E_TYPE",
        "literal",
    );
}

#[test]
fn scalar_rest_is_not_silently_treated_as_a_wildcard() {
    rejected(
        "fn bad(value:u8)->i64{match value{.. =>1}}",
        "E_UNSUPPORTED",
        "range",
    );
}
