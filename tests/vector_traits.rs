//! Production Vec helpers run through both Rust and the ordinary source bundle.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, syntax};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const SEQUENCE: &str = include_str!("../library/sequence.rs");
static NEXT: AtomicUsize = AtomicUsize::new(0);

const COMMON: &str = r#"
fn same_bytes(actual: &String, expected: &str) -> bool {
    let left=actual.as_bytes();let right=expected.as_bytes();
    if left.len()!=right.len(){return false;}
    let mut index:usize=0;
    while index<left.len(){
        if left[index]!=right[index]{return false;}
        index=index+1;
    }
    true
}
fn cloned<T:Clone>(value:&T)->T{value.clone()}
fn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}
fn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}
fn same_operator<T:PartialEq>(left:&T,right:&T)->bool{*left==*right}

struct Token { phase:i64 }
impl Drop for Token {
    fn drop(&mut self) {
        assert!(self.phase==2);
        self.phase=3;
    }
}
struct Item { value:i64, generation:i64, token:Box<Token> }
impl Item {
    fn clone(&self)->Self {
        Item{value:self.value,generation:99,token:Box::new(Token{phase:1})}
    }
}
impl Clone for Item {
    fn clone(&self)->Self {
        assert!(self.token.phase==1);
        Item{value:self.value,generation:self.generation+1,token:Box::new(Token{phase:1})}
    }
}
impl PartialEq for Item {
    fn eq(&self,other:&Self)->bool { self.value==other.value }
    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self,other:&Self)->bool { !self.eq(other) }
}
impl Eq for Item {}
impl Drop for Item {
    fn drop(&mut self) {
        assert!(self.token.phase==1);
        self.token.phase=2;
    }
}
fn item(value:i64)->Item {
    Item{value,generation:0,token:Box::new(Token{phase:1})}
}

pub fn affine_clones_have_independent_storage_and_ordered_cleanup()->i64 {
    let mut original:Vec<Item>=Vec::new();
    original.push(item(4));original.push(item(2));
    let direct=original[0].clone();
    if direct.generation!=99 {return 8;}
    drop(direct);
    let first=original.clone();
    let mut second=cloned(&first);
    if first[0].generation!=1 || first[1].generation!=1 {return 1;}
    if second[0].generation!=2 || second[1].generation!=2 {return 2;}
    if second[0].value!=4 || second[1].value!=2 {return 3;}
    drop(first);
    if original[0].token.phase!=1 || second[0].token.phase!=1 {return 4;}
    original[0].value=7;
    if second[0].value!=4 {return 5;}
    drop(original);
    let last=second.pop().unwrap();
    if last.value!=2 || last.generation!=2 {return 6;}
    drop(last);
    if second[0].token.phase!=1 {return 7;}
    0
}

pub fn equality_uses_element_eq_and_preserves_owners()->i64 {
    let mut left:Vec<Item>=Vec::new();left.push(item(4));left.push(item(2));
    let mut right=cloned(&left);
    if !(left==right) || left!=right {return 1;}
    if !left.eq(&right) || left.ne(&right) {return 2;}
    if !same(&left,&right) || different(&left,&right) {return 3;}
    if !same_operator(&left,&right) {return 4;}
    right[1].value=9;
    if left==right || !(left!=right) {return 5;}
    if same(&left,&right) || !different(&left,&right) {return 6;}
    if !left.ne(&right) {return 7;}
    left.push(item(8));right.pop();
    if left.len()!=3 || right.len()!=1 {return 8;}
    if left==right || right==left || !left.ne(&right) {return 10;}
    if left[0].token.phase!=1 || right[0].token.phase!=1 {return 9;}
    0
}

pub fn nested_vectors_and_strings_clone_as_owned_values()->i64 {
    let empty:Vec<String>=Vec::new();
    if empty.clone().len()!=0 {return 5;}
    let mut inner:Vec<String>=Vec::new();
    inner.push(String::from_str(""));inner.push(String::from_str("a\0水😀"));
    let mut outer:Vec<Vec<String>>=Vec::new();outer.push(inner);
    let copied=cloned(&outer);
    if !same(&outer,&copied) || outer!=copied {return 1;}
    outer[0][1].clear();outer[0][1].push_str("changed");
    if !same_bytes(&copied[0][1],"a\0水😀") {return 2;}
    if outer==copied || !different(&outer,&copied) {return 3;}
    drop(outer);
    if !same_bytes(&copied[0][0],"") || !same_bytes(&copied[0][1],"a\0水😀") {return 4;}
    0
}

pub fn floating_vector_equality_needs_partial_eq_only()->i64 {
    let mut left:Vec<f64>=vec![1.5,-0.0,4.25];
    let mut right:Vec<f64>=vec![1.5,0.0,4.25];
    if left!=right || !same(&left,&right) {return 1;}
    let not_a_number:f64=0.0/0.0;
    let values:Vec<f64>=vec![not_a_number];
    if values==values || same(&values,&values) {return 2;}
    if !values.ne(&values) || !different(&values,&values) {return 3;}
    let empty:Vec<f64>=Vec::new();
    if empty!=empty || !(empty==empty) {return 4;}
    left.push(8.0);right.pop();
    if left.len()!=4 || right.len()!=2 {return 5;}
    0
}

pub fn exact_source_helpers_match_the_ordinary_vector_api()->i64 {
    let mut values:Vec<Item>=Vec::new();values.push(item(1));values.push(item(3));
    let explicit=__lkj_vec_clone(&values);
    let ordinary=values.clone();
    if !__lkj_vec_equal(&explicit,&ordinary) {return 1;}
    if explicit[0].generation!=ordinary[0].generation {return 2;}
    if explicit[1].value!=3 || ordinary[0].value!=1 {return 3;}
    drop(values);drop(ordinary);
    if explicit[0].token.phase!=1 {return 4;}
    0
}
"#;

fn bundle(root: &str, sequence: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files
        .iter_mut()
        .find(|file| file.path == "$std/sequence.rs")
        .unwrap()
        .text = sequence.to_string();
    files.push(SourceInput {
        path: "vector_traits_probe.rs".to_string(),
        text: root.to_string(),
    });
    SourceBundle {
        root: "vector_traits_probe.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn run(artifact: &[u8], expected: &[(String, i64)]) {
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

fn rust_observations(source: &str, probes: &[&str]) -> Vec<(String, i64)> {
    let root = env!("CARGO_MANIFEST_DIR");
    // The String and sequence implementations are the exact production sources.
    let mut rust = String::new();
    for (module, file) in [
        ("prelude", "prelude.rs"),
        ("bytes", "bytes.rs"),
        ("string", "string.rs"),
        ("str", "str.rs"),
        ("sequence", "sequence.rs"),
    ] {
        let path = format!("{root}/library/{file}");
        rust.push_str(&format!("#[path={path:?}] mod {module};\n"));
    }
    rust.push_str(
        "pub use prelude::{Option,Result};use string::String;use sequence::{__lkj_vec_clone,__lkj_vec_equal};\n",
    );
    rust.push_str(source);
    rust.push_str("\nfn main(){\n");
    for probe in probes {
        rust.push_str(&format!("println!(\"{probe} {{}}\",{probe}());\n"));
    }
    rust.push_str("}\n");
    let directory = std::env::temp_dir().join(format!(
        "lkjscript-vector-traits-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("main.rs");
    let binary = directory.join("oracle");
    std::fs::write(&file, rust).unwrap();
    let built = Command::new("rustc")
        .args([
            "--edition=2024",
            "--cap-lints=allow",
            "-C",
            "overflow-checks=yes",
        ])
        .arg(file)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let observed = Command::new(binary).output().unwrap();
    assert!(
        observed.status.success(),
        "{}",
        String::from_utf8_lossy(&observed.stderr)
    );
    let output = String::from_utf8(observed.stdout).unwrap();
    let values = output
        .lines()
        .map(|line| {
            let (name, value) = line.split_once(' ').unwrap();
            (name.to_string(), value.parse().unwrap())
        })
        .collect();
    std::fs::remove_dir_all(directory).unwrap();
    values
}

#[test]
fn owned_vectors_and_exact_helpers_match_rust_with_affine_elements() {
    let probes = [
        "affine_clones_have_independent_storage_and_ordered_cleanup",
        "equality_uses_element_eq_and_preserves_owners",
        "nested_vectors_and_strings_clone_as_owned_values",
        "floating_vector_equality_needs_partial_eq_only",
        "exact_source_helpers_match_the_ordinary_vector_api",
    ];
    let expected = rust_observations(COMMON, &probes);
    assert!(
        expected.iter().all(|(_, value)| *value == 0),
        "{expected:?}"
    );
    let root = format!("use std::{{__lkj_vec_clone,__lkj_vec_equal}};\n{COMMON}");
    let artifact = compiler::compile(&bundle(&root, SEQUENCE)).unwrap();
    run(&artifact, &expected);
}

#[test]
fn vector_equality_uses_eq_even_when_an_element_ne_override_disagrees() {
    let source = r#"
struct Item { value:Box<i64> }
impl PartialEq for Item {
    fn eq(&self,other:&Self)->bool { *self.value==*other.value }
    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self,_other:&Self)->bool { false }
}
fn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}
fn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}
pub fn exact_helper()->i64 {
    let left:Vec<Item>=vec![Item{value:Box::new(1)}];
    let right:Vec<Item>=vec![Item{value:Box::new(2)}];
    if __lkj_vec_equal(&left,&right) {1} else {0}
}
pub fn vector_rule()->i64 {
    let left:Vec<Item>=vec![Item{value:Box::new(1)}];
    let right:Vec<Item>=vec![Item{value:Box::new(2)}];
    if left==right || !(left!=right) {return 1;}
    if left.eq(&right) || !left.ne(&right) {return 2;}
    if same(&left,&right) || !different(&left,&right) {return 3;}
    0
}
"#;
    // This override violates Rust's PartialEq consistency law. Compare the
    // exact source helper under Rust; ordinary Vec's optimization is no oracle
    // for the language rule that vector equality calls element eq.
    let mut expected = rust_observations(source, &["exact_helper"]);
    assert_eq!(expected[0].1, 0);
    expected.push(("vector_rule".to_string(), 0));
    let root = format!("use std::{{__lkj_vec_equal}};\n{source}");
    run(
        &compiler::compile(&bundle(&root, SEQUENCE)).unwrap(),
        &expected,
    );
}

#[test]
fn cloned_string_vectors_preserve_every_byte_after_original_cleanup() {
    let inputs = [
        String::new(),
        "\0".to_string(),
        (0_u8..=127).map(char::from).collect(),
        "α水😀\0\u{10ffff}".to_string(),
        "long\0α水😀".repeat(128),
    ];
    let mut source = COMMON.to_string();
    source.push_str("pub fn all_bytes()->i64{\nlet mut values:Vec<String>=Vec::new();\n");
    for input in &inputs {
        source.push_str(&format!("values.push(String::from_str({input:?}));\n"));
    }
    source.push_str("let copied=values.clone();let copied_twice=cloned(&copied);drop(values);\n");
    for (index, input) in inputs.iter().enumerate() {
        source.push_str(&format!(
            "if !same_bytes(&copied[{index}],{input:?}){{return {};}}\n",
            index + 1
        ));
    }
    source.push_str("drop(copied);\n");
    for (index, input) in inputs.iter().enumerate() {
        source.push_str(&format!(
            "if !same_bytes(&copied_twice[{index}],{input:?}){{return {};}}\n",
            inputs.len() + index + 1
        ));
    }
    source.push_str("0}\n");
    let expected = rust_observations(&source, &["all_bytes"]);
    assert_eq!(expected[0].1, 0);
    let root = format!("use std::{{__lkj_vec_clone,__lkj_vec_equal}};\n{source}");
    run(
        &compiler::compile(&bundle(&root, SEQUENCE)).unwrap(),
        &expected,
    );
}

#[test]
fn element_constraints_and_existing_view_storage_rules_are_enforced() {
    for (source, code, fragment) in [
        (
            "fn bad<T>(value:&Vec<T>)->Vec<T>{value.clone()}",
            "E_TRAIT",
            "Clone",
        ),
        (
            "fn bad<T>(left:&Vec<T>,right:&Vec<T>)->bool{left==right}",
            "E_TRAIT",
            "PartialEq",
        ),
        (
            "struct Item;fn probe(){let values:Vec<Item>=Vec::new();values.clone();}",
            "E_TRAIT",
            "Clone",
        ),
        (
            "struct Item;fn probe()->bool{let values:Vec<Item>=Vec::new();values==values}",
            "E_TRAIT",
            "PartialEq",
        ),
        (
            "fn probe(){let values:Vec<i64>=Vec::new();values.clone::<bool>();}",
            "E_TYPE",
            "generic",
        ),
        (
            "fn probe()->bool{let values:Vec<i64>=Vec::new();let wrong:i64=1;values.eq(wrong)}",
            "E_TYPE",
            "argument",
        ),
        (
            "fn probe(){let value:i64=1;let values=vec![&value];values.clone();}",
            "E_BORROW",
            "views cannot be stored",
        ),
    ] {
        let error = compiler::check_bundle(&bundle(source, SEQUENCE))
            .err()
            .expect("invalid vector operation must be rejected");
        assert_eq!(error.code, code, "{source}\n{error:?}");
        assert!(error.message.contains(fragment), "{source}\n{error:?}");
    }
}

#[test]
fn vector_helpers_have_source_library_identity() {
    let source = r#"
fn __lkj_vec_clone<T:Clone>(_value:&Vec<T>)->Vec<T>{Vec::new()}
fn __lkj_vec_equal<T:PartialEq>(_left:&Vec<T>,_right:&Vec<T>)->bool{false}
mod std {
    pub fn __lkj_vec_clone<T:Clone>(_value:&Vec<T>)->Vec<T>{Vec::new()}
    pub fn __lkj_vec_equal<T:PartialEq>(_left:&Vec<T>,_right:&Vec<T>)->bool{false}
}
pub fn probe()->i64 {
    let values:Vec<i64>=vec![4,2];let copied=values.clone();
    if copied==values {copied[0]*10+copied[1]} else {0}
}
"#;
    run(
        &compiler::compile(&bundle(source, SEQUENCE)).unwrap(),
        &[("probe".to_string(), 42)],
    );
    let parsed = syntax::parse(source).unwrap();
    let error = compiler::check_source(&parsed)
        .err()
        .expect("a same-spelled user helper must not replace a missing library");
    assert_eq!(error.code, "E_UNSUPPORTED", "{error:?}");
    assert!(
        error
            .message
            .contains("source library module $std::sequence is not loaded"),
        "{error:?}"
    );
}

#[test]
fn ordinary_vector_methods_include_builtin_traits_in_scope_resolution() {
    for (trait_name, method, result, arguments, body, call) in [
        (
            "Other",
            "clone",
            "Vec<i64>",
            "",
            "Vec::new()",
            "values.clone()",
        ),
        (
            "Clone",
            "clone",
            "Vec<i64>",
            "",
            "Vec::new()",
            "values.clone()",
        ),
        (
            "Other",
            "eq",
            "bool",
            ",other:&Self",
            "false",
            "values.eq(&values)",
        ),
        (
            "PartialEq",
            "eq",
            "bool",
            ",other:&Self",
            "false",
            "values.eq(&values)",
        ),
        (
            "Other",
            "ne",
            "bool",
            ",other:&Self",
            "false",
            "values.ne(&values)",
        ),
    ] {
        let source = format!(
            "trait {trait_name}{{fn {method}(&self{arguments})->{result};}} impl {trait_name} for Vec<i64>{{fn {method}(&self{arguments})->{result}{{{body}}}}} fn probe(){{let values:Vec<i64>=Vec::new();{call};}}"
        );
        let error = compiler::check_bundle(&bundle(&source, SEQUENCE))
            .err()
            .expect("both applicable trait methods must be considered");
        assert_eq!(error.code, "E_TRAIT", "{source}\n{error:?}");
        assert!(error.message.contains("ambiguous"), "{source}\n{error:?}");
    }
    let source = r#"
mod extra {
    pub trait Other {
        fn clone(&self)->Vec<i64>;
        fn eq(&self,other:&Self)->bool;
        fn ne(&self,other:&Self)->bool;
    }
    impl Other for Vec<i64> {
        fn clone(&self)->Vec<i64>{vec![9]}
        fn eq(&self,_other:&Self)->bool{false}
        fn ne(&self,_other:&Self)->bool{false}
    }
}
fn selected<T:extra::Other>(value:&T)->i64 {
    let copied=value.clone();
    if value.eq(value) || value.ne(value) {return 0;}
    copied[0]
}
pub fn scope_probe()->i64 {
    let values:Vec<i64>=vec![4,2];let copied=values.clone();
    if copied!=values || !values.eq(&values) || values.ne(&values) {return 0;}
    copied[0]*10+copied[1]+selected(&values)
}
"#;
    let expected = rust_observations(source, &["scope_probe"]);
    assert_eq!(expected[0].1, 51);
    run(
        &compiler::compile(&bundle(source, SEQUENCE)).unwrap(),
        &expected,
    );

    let without_builtin = r#"
struct NoClone;
trait Other {fn clone(&self)->Vec<i64>;}
impl Other for Vec<NoClone>{fn clone(&self)->Vec<i64>{vec![9]}}
pub fn only_source_trait()->i64 {
    let values:Vec<NoClone>=Vec::new();values.clone()[0]
}
"#;
    run(
        &compiler::compile(&bundle(without_builtin, SEQUENCE)).unwrap(),
        &[("only_source_trait".to_string(), 9)],
    );
}

#[test]
fn production_sequence_mutations_change_vector_artifacts_and_behavior() {
    let root = r#"
pub fn clone_probe()->i64 {
    let values:Vec<i64>=vec![4,2];let copied=values.clone();
    copied[0]*10+copied[1]
}
pub fn eq_probe()->i64 {
    let left:Vec<i64>=vec![4,2];let right:Vec<i64>=vec![4,3];
    if left==right {1} else {0}
}
pub fn ne_probe()->i64 {
    let left:Vec<i64>=vec![4,2];let right:Vec<i64>=vec![4,3];
    if left.ne(&right) {1} else {0}
}
"#;
    let original = compiler::compile(&bundle(root, SEQUENCE)).unwrap();
    run(
        &original,
        &[
            ("clone_probe".to_string(), 42),
            ("eq_probe".to_string(), 0),
            ("ne_probe".to_string(), 1),
        ],
    );
    let clone_changed = SEQUENCE.replace(
        "result.push(source[index].clone());",
        "result.push(source[0].clone());",
    );
    assert_ne!(clone_changed, SEQUENCE);
    let cloned = compiler::compile(&bundle(root, &clone_changed)).unwrap();
    assert_ne!(cloned, original);
    run(&cloned, &[("clone_probe".to_string(), 44)]);
    let equality_changed = SEQUENCE.replace("if !(left[index] == right[index]) {", "if false {");
    assert_ne!(equality_changed, SEQUENCE);
    let compared = compiler::compile(&bundle(root, &equality_changed)).unwrap();
    assert_ne!(compared, original);
    run(
        &compared,
        &[("eq_probe".to_string(), 1), ("ne_probe".to_string(), 0)],
    );
}

#[test]
fn invalid_sequence_source_mutation_keeps_the_library_diagnostic() {
    let changed = SEQUENCE.replace("result.push(source[index].clone());", "result.push(true);");
    assert_ne!(changed, SEQUENCE);
    let error = compiler::compile(&bundle(
        "fn probe(){let values:Vec<i64>=Vec::new();values.clone();}",
        &changed,
    ))
    .unwrap_err();
    assert_eq!(error.code, "E_TYPE", "{error:?}");
    assert_eq!(error.path, "$std/sequence.rs", "{error:?}");
}

#[test]
fn weakened_library_helpers_cannot_grant_missing_element_contracts() {
    let clone_start = SEQUENCE.find("pub fn __lkj_vec_clone").unwrap();
    let equality_start = SEQUENCE.find("pub fn __lkj_vec_equal").unwrap();
    let repeat_start = SEQUENCE.find("pub fn __lkj_vec_repeat").unwrap();
    let weak_clone = format!(
        "{}pub fn __lkj_vec_clone<T>(_source:&Vec<T>)->Vec<T>{{Vec::new()}}\n{}",
        &SEQUENCE[..clone_start],
        &SEQUENCE[equality_start..]
    );
    let weak_equality = format!(
        "{}pub fn __lkj_vec_equal<T>(_left:&Vec<T>,_right:&Vec<T>)->bool{{true}}\n{}",
        &SEQUENCE[..equality_start],
        &SEQUENCE[repeat_start..]
    );
    for (changed, source, required) in [
        (
            &weak_clone,
            "struct Item;fn probe(){let values:Vec<Item>=Vec::new();values.clone();}",
            "Clone",
        ),
        (
            &weak_equality,
            "struct Item;fn probe()->bool{let values:Vec<Item>=Vec::new();values==values}",
            "PartialEq",
        ),
    ] {
        let error = compiler::check_bundle(&bundle(source, changed))
            .err()
            .expect("a helper body does not grant a builtin trait");
        assert_eq!(error.code, "E_TRAIT", "{error:?}");
        assert!(error.message.contains(required), "{error:?}");
        assert_eq!(error.path, "vector_traits_probe.rs", "{error:?}");
    }
    let wrong_result = format!(
        "{}pub fn __lkj_vec_clone<T:Clone>(_source:&Vec<T>)->bool{{true}}\n{}",
        &SEQUENCE[..clone_start],
        &SEQUENCE[equality_start..]
    );
    let error = compiler::check_bundle(&bundle(
        "fn probe(){let values:Vec<i64>=Vec::new();values.clone();}",
        &wrong_result,
    ))
    .err()
    .expect("helper result must match the operation");
    assert_eq!(error.code, "E_TYPE", "{error:?}");
    assert!(error.message.contains("helper signature"), "{error:?}");
}

#[test]
fn derived_holders_clone_their_vector_storage() {
    let source = r#"
#[derive(Clone)] struct Holder {values:Vec<i64>}
pub fn derived_holder()->i64 {
    let empty=Holder{values:Vec::new()};
    let copied_empty=empty.clone();
    drop(empty);
    if copied_empty.values.len()!=0 {return 1;}
    let mut original=Holder{values:vec![4,2]};
    let copied=original.clone();
    original.values[0]=9;
    drop(original);
    if copied.values.len()!=2 || copied.values[0]!=4 || copied.values[1]!=2 {return 2;}
    0
}
"#;
    let expected = rust_observations(source, &["derived_holder"]);
    assert_eq!(expected[0].1, 0);
    run(
        &compiler::compile(&bundle(source, SEQUENCE)).unwrap(),
        &expected,
    );
}
