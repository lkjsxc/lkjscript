//! Generic substitution follows the declared parameter scope and item identity.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, k};
use std::io::Write;
use std::process::{Command, Stdio};

fn bundle(source: &str, library: bool) -> SourceBundle {
    let mut files = if library {
        bootstrap_library::standard_library()
    } else {
        Vec::new()
    };
    files.push(SourceInput {
        path: "scope.rs".to_string(),
        text: source.to_string(),
    });
    SourceBundle {
        root: "scope.rs".to_string(),
        library: if library {
            Some("$std/lib.rs".to_string())
        } else {
            None
        },
        files,
    }
}

fn check(bundle: &SourceBundle) -> k::CheckedProgram {
    compiler::check_bundle(bundle)
        .unwrap_or_else(|problem| panic!("valid generic scope rejected: {problem:?}"))
}

fn execute(bundle: &SourceBundle, expected: i64) {
    let checked = check(bundle);
    let artifact = lkjscript::wasm::emit(&checked).unwrap();
    let script = format!(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{{try{{\
         const {{instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),{{lkjscript:{{trap(c,l){{throw Error('trap '+c+':'+l)}}}}}});\
         const actual=instance.exports['fn:probe']();if(actual!=={expected}n)throw Error('result '+actual);\
         }}catch(error){{console.error(error);process.exitCode=1}}}});"
    );
    let mut child = Command::new("node")
        .args(["-e", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&artifact).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn reject(source: &str) {
    let error = match compiler::check_bundle(&bundle(source, false)) {
        Ok(_) => panic!("a declaration was substituted as a generic parameter"),
        Err(problem) => problem,
    };
    assert_eq!(error.code, "E_TYPE", "{error:?}");
    assert_eq!(error.path, "scope.rs", "{error:?}");
}

#[test]
fn user_names_do_not_change_source_library_generic_method_selection() {
    for name in ["E", "T", "U", "Packet"] {
        let source = format!(
            "enum {name}{{Nothing}} pub fn probe()->usize{{let value=\"scope\".to_string();value.as_str().len()}}"
        );
        execute(&bundle(&source, true), 5);
    }
}

#[test]
fn declared_parameters_shadow_unrelated_nominal_names() {
    let cases = [
        (
            "struct T; fn identity<T>(value:T)->T{value} pub fn probe()->i64{identity(7i64)}",
            7,
        ),
        (
            "enum E{Nothing} struct Holder<E>{value:E} impl<E> Holder<E>{fn take(self)->E{self.value}} pub fn probe()->i64{let value:Holder<i64>=Holder{value:9};value.take()}",
            9,
        ),
        (
            "struct E; struct Counter(i64); impl Counter{fn echo<E>(&self,value:E)->E{value}} pub fn probe()->i64{let counter=Counter(0);counter.echo(11i64)}",
            11,
        ),
        (
            "fn identity<str>(value:str)->str{value} pub fn probe()->i64{identity(13i64)}",
            13,
        ),
    ];
    for (source, expected) in cases {
        execute(&bundle(source, false), expected);
    }
}

#[test]
fn qualified_nominal_types_keep_their_declaration_inside_generic_scopes() {
    let source = "struct T(i64); fn combine<T>(named:crate::T,value:T)->(crate::T,T){(named,value)} pub fn probe()->i64{let (named,value)=combine::<i64>(T(4),6i64);named.0+value}";
    execute(&bundle(source, false), 10);
}

#[test]
fn parameter_scope_uses_the_original_file() {
    let mut input = bundle(
        "struct T(i64); mod generic; pub fn probe()->i64{generic::identity(17i64)}",
        false,
    );
    input.files.push(SourceInput {
        path: "generic.rs".to_string(),
        text: "pub fn identity<T>(value:T)->T{value}".to_string(),
    });
    execute(&input, 17);
}

#[test]
fn builtin_text_cannot_be_inferred_as_a_parameter() {
    reject(
        "fn text_only(value:&str)->i64{7} pub fn probe()->i64{let value=1i64;text_only(&value)}",
    );
}

#[test]
fn explicit_arguments_do_not_substitute_a_qualified_nominal_type() {
    reject(
        "struct T(i64); fn accept<T>(named:crate::T,value:T)->i64{5} pub fn probe()->i64{accept::<i64>(1i64,2i64)}",
    );
}

#[test]
fn a_type_parameter_cannot_be_applied_as_a_type_constructor() {
    reject("fn invalid<T>(value:T<i64>){} pub fn probe()->i64{0}");
}

fn rust_check(label: &str, source: &str, accepted: bool) -> String {
    let directory =
        std::path::Path::new(".artifacts").join(format!("generic-scope-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{label}.rs"));
    let metadata = directory.join(format!("{label}.rmeta"));
    std::fs::write(&path, source).unwrap();
    let output = Command::new("rustc")
        .args([
            "--edition=2024",
            "--cap-lints=allow",
            "--crate-type=lib",
            "--emit=metadata",
        ])
        .arg(&path)
        .arg("-o")
        .arg(&metadata)
        .output()
        .expect("rustc is required for the generic-scope oracle");
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.success(), accepted, "{diagnostic}");
    diagnostic
}

#[test]
fn resolved_nominal_paths_keep_generic_implementations_disjoint() {
    let cases = [
        "pub struct T(pub i64); trait Marker<U>{} impl<T> Marker<T> for crate::T{} impl<U> Marker<U> for i64{} pub fn probe()->i64{0}",
        "pub struct T(pub i64); mod names{pub use crate::T as Target;} trait Marker<U>{} impl<T> Marker<T> for names::Target{} impl<U> Marker<U> for i64{} pub fn probe()->i64{0}",
        "pub struct T(pub i64); mod sibling{pub trait Marker<U>{}} impl<T> sibling::Marker<T> for crate::T{} impl<U> sibling::Marker<U> for i64{} pub fn probe()->i64{0}",
    ];
    for (index, source) in cases.iter().enumerate() {
        check(&bundle(source, false));
        rust_check(&format!("disjoint_{index}"), source, true);
    }
}

#[test]
fn parameter_name_does_not_grant_a_trait_to_another_declaration() {
    let prefix = "struct T(i64); trait Marker{} impl<T> Marker for (crate::T,T){} fn require<X:Marker>(value:X)->i64{7}";
    let good = format!("{prefix} pub fn probe()->i64{{require((T(1),2i64))}}");
    execute(&bundle(&good, false), 7);
    rust_check("right_witness", &good, true);
    let bad = format!("{prefix} pub fn probe()->i64{{require((1i64,2i64))}}");
    let error = match compiler::check_bundle(&bundle(&bad, false)) {
        Ok(_) => panic!("a nominal implementation was accepted for an unrelated scalar"),
        Err(problem) => problem,
    };
    assert_eq!(error.code, "E_TRAIT", "{error:?}");
    assert_eq!(error.path, "scope.rs", "{error:?}");
    assert!(rust_check("wrong_witness", &bad, false).contains("E0277"));
}
