//! The exact production String implementation runs under rustc and lkjscript.
#[path = "../bridge/library.rs"]
mod bootstrap_library;
#[path = "../library/bytes.rs"]
#[allow(dead_code)]
mod bytes;
#[path = "../library/prelude.rs"]
#[allow(dead_code)]
pub mod rust_prelude;
#[path = "../library/string.rs"]
#[allow(dead_code)]
mod rust_string;
#[path = "../library/str.rs"]
#[allow(dead_code)]
mod str;
mod string {
    pub use super::rust_string::String;
}
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

use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
use rust_string::String as OwnedString;
use std::io::Write;
use std::process::{Command, Stdio};

const STRING: &str = include_str!("../library/string.rs");
const BYTE_COMPARISON: &str = r#"
fn same_bytes(value: &String, expected: &str) -> bool {
    let actual = value.as_bytes();
    let expected_bytes = expected.as_bytes();
    if actual.len() != expected_bytes.len() { return false; }
    let mut index: usize = 0;
    while index < actual.len() {
        if actual[index] != expected_bytes[index] { return false; }
        index = index + 1;
    }
    true
}
"#;

fn bundle(root: &str, string_source: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    let library = files
        .iter_mut()
        .find(|file| file.path == "$std/string.rs")
        .unwrap();
    library.text = string_source.to_string();
    files.push(SourceInput {
        path: "string_traits_probe.rs".to_string(),
        text: root.to_string(),
    });
    SourceBundle {
        root: "string_traits_probe.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn run(artifact: &[u8], expected: &[(&str, usize)]) {
    let mut script = String::from(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{try{const {instance}=await WebAssembly.instantiate(Buffer.concat(chunks),{lkjscript:{trap(c,l){throw Error('trap '+c+':'+l)}}});",
    );
    for (entry, value) in expected {
        script.push_str(&format!(
            "{{const actual=instance.exports['fn:{entry}']();if(actual!=={value}n)throw Error('{entry}: expected {value}, got '+actual);}}"
        ));
    }
    script.push_str("}catch(e){console.error(e);process.exitCode=1}})");
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(artifact).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn inputs() -> Vec<String> {
    vec![
        String::new(),
        "a".to_string(),
        "ab".to_string(),
        "b".to_string(),
        "\0".to_string(),
        (0_u8..=127).map(char::from).collect(),
        "α水😀\0\u{10ffff}".to_string(),
        "long\0α水😀".repeat(128),
    ]
}

fn expect_bytes(source: &mut String, check: &mut usize, name: &str, expected: &OwnedString) {
    let expected = std::str::from_utf8(expected.as_bytes()).unwrap();
    source.push_str(&format!(
        "if !same_bytes(&{name}, {expected:?}) {{ return {check}; }}\n"
    ));
    *check += 1;
}

#[test]
fn clones_and_clone_from_match_rust_over_every_byte_and_keep_independent_owners() {
    let mut source = BYTE_COMPARISON.to_string();
    let mut entries = Vec::new();
    let suffix = "original\0😀";
    let other_suffix = "clone\u{7f}水";
    for (index, input) in inputs().iter().enumerate() {
        let entry = format!("lifecycle_{index}");
        entries.push(entry.clone());
        let mut check = 1;
        source.push_str(&format!(
            "pub fn {entry}() -> usize {{\nlet mut original = String::from_str({input:?});\nlet mut cloned = original.clone();\n"
        ));
        let mut original = OwnedString::from_str(input);
        let mut cloned = original.clone();
        assert_eq!(cloned.as_bytes(), input.as_bytes());
        expect_bytes(&mut source, &mut check, "original", &original);
        expect_bytes(&mut source, &mut check, "cloned", &cloned);

        source.push_str(&format!(
            "original.clear(); original.push_str({suffix:?});\n"
        ));
        original.clear();
        original.push_str(suffix);
        expect_bytes(&mut source, &mut check, "original", &original);
        expect_bytes(&mut source, &mut check, "cloned", &cloned);

        source.push_str(&format!(
            "cloned.clear(); cloned.push_str({other_suffix:?});\n"
        ));
        cloned.clear();
        cloned.push_str(other_suffix);
        expect_bytes(&mut source, &mut check, "original", &original);
        expect_bytes(&mut source, &mut check, "cloned", &cloned);

        source.push_str("let survivor = original.clone(); drop(original);\n");
        let survivor = original.clone();
        drop(original);
        expect_bytes(&mut source, &mut check, "survivor", &survivor);
        source.push_str("drop(cloned); let discarded = survivor.clone(); drop(discarded);\n");
        drop(cloned);
        let discarded = survivor.clone();
        drop(discarded);
        expect_bytes(&mut source, &mut check, "survivor", &survivor);

        source.push_str("let mut target = String::from_str(\"preexisting allocation\");\n");
        let mut target = OwnedString::from_str("preexisting allocation");
        source.push_str(&format!(
            "{{ let temporary = String::from_str({input:?}); target.clone_from(&temporary); }}\n"
        ));
        {
            let temporary = OwnedString::from_str(input);
            target.clone_from(&temporary);
        }
        expect_bytes(&mut source, &mut check, "target", &target);
        source.push_str("target.clone_from(&survivor);\n");
        target.clone_from(&survivor);
        expect_bytes(&mut source, &mut check, "target", &target);
        source.push_str("let empty = String::new(); target.clone_from(&empty);\n");
        let empty = OwnedString::new();
        target.clone_from(&empty);
        expect_bytes(&mut source, &mut check, "target", &target);

        // Repeated replacement includes growth, shrinkage, and empty input.
        source.push_str(&format!(
            "let long = String::from_str({input:?}); let short = String::from_str(\"s\");\n\
             let mut round:usize=0; while round<16 {{\n\
             target.clone_from(&long); if !same_bytes(&target,{input:?}) {{return {check};}}\n"
        ));
        check += 1;
        source.push_str(&format!(
            "target.clone_from(&short); if !same_bytes(&target,\"s\") {{return {check};}}\n"
        ));
        check += 1;
        source.push_str(&format!(
            "target.clone_from(&empty); if !same_bytes(&target,\"\") {{return {check};}}\n\
             round=round+1; }}\n"
        ));
        let long = OwnedString::from_str(input);
        let short = OwnedString::from_str("s");
        for _ in 0..16 {
            target.clone_from(&long);
            assert_eq!(target.as_bytes(), long.as_bytes());
            target.clone_from(&short);
            assert_eq!(target.as_bytes(), b"s");
            target.clone_from(&empty);
            assert!(target.is_empty());
        }
        source.push_str("target.clone_from(&survivor); drop(survivor); target.push_str(\"!\");\n");
        target.clone_from(&survivor);
        drop(survivor);
        target.push_str("!");
        expect_bytes(&mut source, &mut check, "target", &target);
        source.push_str("0\n}\n");
    }
    let artifact = compiler::compile(&bundle(&source, STRING)).unwrap();
    let expected: Vec<_> = entries.iter().map(|entry| (entry.as_str(), 0)).collect();
    run(&artifact, &expected);

    // The same production method retains its Vec allocation when capacity suffices.
    // The owned execution above establishes lifecycle behavior, not an address ABI.
    let mut allocated = OwnedString::from_str(&"retained storage".repeat(64));
    let storage = allocated.as_bytes().as_ptr();
    allocated.clone_from(&OwnedString::from_str("short"));
    assert_eq!(allocated.as_bytes().as_ptr(), storage);
}

#[test]
fn equality_methods_match_rust_and_borrow_both_owners() {
    let cases = inputs();
    let mut source = BYTE_COMPARISON.to_string();
    let mut entries = Vec::new();
    for (i, left) in cases.iter().enumerate() {
        for (j, right) in cases.iter().enumerate() {
            let entry = format!("equality_{i}_{j}");
            entries.push(entry.clone());
            let mut a = OwnedString::from_str(left);
            let mut b = OwnedString::from_str(right);
            let equal = a.eq(&b);
            let unequal = a.ne(&b);
            assert_eq!(equal, left.as_bytes() == right.as_bytes());
            assert_eq!(unequal, left.as_bytes() != right.as_bytes());
            source.push_str(&format!(
                "pub fn {entry}()->usize {{\n\
                 let mut a=String::from_str({left:?}); let mut b=String::from_str({right:?});\n\
                 if a.eq(&b)!={equal} {{return 1;}}\n\
                 if a.ne(&b)!={unequal} {{return 2;}}\n\
                 if (a==b)!={equal} {{return 6;}}\n\
                 if (a!=b)!={unequal} {{return 7;}}\n\
                 if !a.eq(&a) {{return 3;}}\n\
                 a.push_str(\"!\"); b.clear(); b.push_str(\"reused\\0水\");\n"
            ));
            a.push_str("!");
            b.clear();
            b.push_str("reused\0水");
            let mut check = 4;
            expect_bytes(&mut source, &mut check, "a", &a);
            expect_bytes(&mut source, &mut check, "b", &b);
            source.push_str("0\n}\n");
        }
    }
    let artifact = compiler::compile(&bundle(&source, STRING)).unwrap();
    let expected: Vec<_> = entries.iter().map(|entry| (entry.as_str(), 0)).collect();
    run(&artifact, &expected);
}

#[test]
fn generic_clone_and_equality_use_declared_builtin_witnesses() {
    let source = format!(
        "{BYTE_COMPARISON}\n{}",
        r#"
fn copied<T: Clone>(value: &T) -> T { value.clone() }
fn copied_from<T: Clone>(target: &mut T, source: &T) { target.clone_from(source); }
fn same<T: PartialEq>(left: &T, right: &T) -> bool { left.eq(right) }
fn different<T: PartialEq>(left: &T, right: &T) -> bool { left.ne(right) }
fn reflexive<T: Eq>(value: &T) -> bool { value.eq(value) }
fn copied_scalar<T: Copy>(value: &T) -> T { value.clone() }
fn same_operator<T: PartialEq>(left: &T, right: &T) -> bool { *left == *right }
fn different_operator<T: PartialEq>(left: &T, right: &T) -> bool { *left != *right }
pub fn generic_probe() -> usize {
    let original=String::from_str("generic\0水😀");
    let mut copy=copied(&original);
    if !same_bytes(&copy,"generic\0水😀") {return 1;}
    if !same(&copy,&original) || different(&copy,&original) {return 2;}
    if !reflexive(&copy) {return 3;}
    if !same_operator(&copy,&original) || different_operator(&copy,&original) {return 5;}
    if copied_scalar(&42i64)!=42 {return 6;}
    copy.clear();
    copied_from(&mut copy,&original);
    drop(original);
    if !same_bytes(&copy,"generic\0水😀") {return 4;}
    0
}
"#
    );
    let artifact = compiler::compile(&bundle(&source, STRING)).unwrap();
    run(&artifact, &[("generic_probe", 0)]);
}

#[test]
fn clone_from_rejects_overlapping_borrows_and_views_cannot_outlive_clones() {
    for source in [
        "fn probe(){let mut value=String::from_str(\"owner\"); value.clone_from(&value);}",
        "fn probe()->u8{let mut value=String::from_str(\"owner\");let view=value.as_bytes();let other=String::from_str(\"other\");value.clone_from(&other);view[0]}",
        "fn bad<'a>(value:&'a String)->&'a [u8]{let copied=value.clone();copied.as_bytes()}",
    ] {
        let error = compiler::check_bundle(&bundle(source, STRING))
            .err()
            .expect("invalid borrow must be rejected");
        assert_eq!(error.code, "E_BORROW", "{source}\n{error:?}");
        assert_eq!(error.path, "string_traits_probe.rs", "{error:?}");
    }
}

#[test]
fn production_clone_and_equality_mutations_change_artifacts_and_results() {
    let source = format!(
        "{BYTE_COMPARISON}\n{}",
        r#"
pub fn original_clone()->usize {
    let value=String::from_str("source").clone();
    if same_bytes(&value,"source") {0} else {1}
}
pub fn changed_clone()->usize {
    let value=String::from_str("source").clone();
    if same_bytes(&value,"changed") {0} else {1}
}
pub fn equal_probe()->usize {
    let left=String::from_str("same");let right=String::from_str("same");
    if left.eq(&right) {1} else {0}
}
pub fn unequal_probe()->usize {
    let left=String::from_str("same");let right=String::from_str("same");
    if left.ne(&right) {1} else {0}
}
pub fn equal_operator()->usize {
    let left=String::from_str("same");let right=String::from_str("same");
    if left==right {1} else {0}
}
pub fn unequal_operator()->usize {
    let left=String::from_str("same");let right=String::from_str("same");
    if left!=right {1} else {0}
}
pub fn differing_method()->usize {
    let left=String::from_str("left");let right=String::from_str("right");
    if left.ne(&right) {1} else {0}
}
pub fn differing_operator()->usize {
    let left=String::from_str("left");let right=String::from_str("right");
    if left!=right {1} else {0}
}
"#
    );
    let original = compiler::compile(&bundle(&source, STRING)).unwrap();
    run(
        &original,
        &[
            ("original_clone", 0),
            ("changed_clone", 1),
            ("equal_probe", 1),
            ("unequal_probe", 0),
            ("equal_operator", 1),
            ("unequal_operator", 0),
            ("differing_method", 1),
            ("differing_operator", 1),
        ],
    );
    let changed_clone = STRING.replace(
        "append_bytes(&mut bytes, self.bytes.as_slice());",
        "append_bytes(&mut bytes, \"changed\".as_bytes());",
    );
    assert_ne!(STRING, changed_clone);
    let clone_artifact = compiler::compile(&bundle(&source, &changed_clone)).unwrap();
    assert_ne!(original, clone_artifact);
    run(
        &clone_artifact,
        &[
            ("original_clone", 1),
            ("changed_clone", 0),
            ("equal_probe", 1),
            ("unequal_probe", 0),
            ("equal_operator", 1),
            ("unequal_operator", 0),
            ("differing_method", 1),
            ("differing_operator", 1),
        ],
    );
    let changed_equality = STRING.replace(
        "self.bytes.len() == other.bytes.len()\n            && starts_with(self.bytes.as_slice(), other.bytes.as_slice())",
        "false",
    );
    assert_ne!(STRING, changed_equality);
    let equality_artifact = compiler::compile(&bundle(&source, &changed_equality)).unwrap();
    assert_ne!(original, equality_artifact);
    run(
        &equality_artifact,
        &[
            ("original_clone", 0),
            ("changed_clone", 1),
            ("equal_probe", 0),
            ("unequal_probe", 1),
            ("equal_operator", 0),
            ("unequal_operator", 1),
            ("differing_method", 1),
            ("differing_operator", 1),
        ],
    );
    let changed_ne = STRING.replace("!self.eq(other)", "false");
    assert_ne!(STRING, changed_ne);
    let ne_artifact = compiler::compile(&bundle(&source, &changed_ne)).unwrap();
    assert_ne!(original, ne_artifact);
    run(
        &ne_artifact,
        &[
            ("equal_operator", 1),
            ("differing_method", 0),
            ("differing_operator", 0),
        ],
    );
}

#[test]
fn invalid_production_clone_mutation_reports_the_library_file() {
    let changed = STRING.replace(
        "append_bytes(&mut bytes, self.bytes.as_slice());",
        "append_bytes(&mut bytes, true);",
    );
    assert_ne!(STRING, changed);
    let error = compiler::compile(&bundle(
        "fn probe()->usize{String::from_str(\"owned\").clone().len()}",
        &changed,
    ))
    .unwrap_err();
    assert_eq!(error.code, "E_TYPE", "{error:?}");
    assert_eq!(error.path, "$std/string.rs", "{error:?}");
}

#[test]
fn unused_generic_builtin_methods_enforce_bounds_types_and_ownership() {
    for (source, code) in [
        ("fn missing<T>(value:&T)->T{value.clone()}", "E_TRAIT"),
        (
            "fn missing<T>(left:&T,right:&T)->bool{left.eq(right)}",
            "E_TRAIT",
        ),
        ("fn missing<T:Eq>(value:&T)->T{value.clone()}", "E_TRAIT"),
        ("fn wrong<T:Clone>(value:&T)->bool{value.clone()}", "E_TYPE"),
        (
            "fn wrong<T:Clone>(value:&T)->T{value.clone(1i64)}",
            "E_TYPE",
        ),
        (
            "fn wrong<T:Clone>(value:&T)->T{value.clone::<i64>()}",
            "E_TYPE",
        ),
        (
            "fn wrong<T:Clone>(target:&mut T,source:&T)->T{target.clone_from(source)}",
            "E_TYPE",
        ),
        (
            "fn wrong<T:PartialEq>(value:&T)->bool{value.eq()}",
            "E_TYPE",
        ),
        (
            "fn wrong<T:PartialEq>(left:&T,right:&i64)->bool{left.eq(right)}",
            "E_TYPE",
        ),
        (
            "fn wrong<T:PartialEq>(left:&T,right:&T)->T{left.eq(right)}",
            "E_TYPE",
        ),
        (
            "fn twice<T:Clone>(value:&T)->(T,T){let copied=value.clone();(copied,copied)}",
            "E_MOVE",
        ),
        (
            "fn overlapping<T:Clone>(value:&mut T){value.clone_from(value);}",
            "E_BORROW",
        ),
    ] {
        let error = compiler::check_bundle(&bundle(source, STRING))
            .err()
            .expect("invalid unused generic method must be rejected");
        assert_eq!(error.code, code, "{source}\n{error:?}");
        assert_eq!(error.path, "string_traits_probe.rs", "{error:?}");
    }
}

#[test]
fn same_spelled_source_traits_keep_their_own_method_contracts() {
    let declarations = r#"
trait Clone {fn clone(&self)->i64;}
trait PartialEq {fn eq(&self,other:&Self)->i64;fn ne(&self,other:&Self)->i64;}
struct Item(i64);
impl Clone for Item {fn clone(&self)->i64{self.0+30}}
impl PartialEq for Item {
    fn eq(&self,other:&Self)->i64{self.0+other.0}
    fn ne(&self,other:&Self)->i64{self.0+other.0+100}
}
fn copy_value<T:Clone>(value:&T)->i64{value.clone()}
fn equal_value<T:PartialEq>(left:&T,right:&T)->i64{left.eq(right)}
fn unequal_value<T:PartialEq>(left:&T,right:&T)->i64{left.ne(right)}
pub fn custom_probe()->i64 {
    let left=Item(4);let right=Item(5);
    copy_value(&left)+equal_value(&left,&right)+unequal_value(&left,&right)
}
"#;
    let artifact = compiler::compile(&bundle(declarations, STRING)).unwrap();
    run(&artifact, &[("custom_probe", 152)]);
    for source in [
        "trait Clone{fn clone(&self)->i64;} fn wrong<T:Clone>(value:&T)->T{value.clone()}",
        "trait PartialEq{fn eq(&self,other:&Self)->i64;} fn wrong<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}",
    ] {
        let error = compiler::check_bundle(&bundle(source, STRING))
            .err()
            .expect("source method contract must not become a builtin by spelling");
        assert_eq!(error.code, "E_TYPE", "{source}\n{error:?}");
    }
}
