//! Generic method checking must preserve its meaning through specialization.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, syntax, wasm};
use std::io::Write;
use std::process::{Command, Stdio};

fn check(source: &str) -> Result<lkjscript::k::CheckedProgram, lkjscript::ast::Diagnostic> {
    compiler::check_source(&syntax::parse(source).expect("fixture parses"))
}

fn run(artifact: &[u8], entries: &[(&str, i64)]) {
    let mut script = String::from(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{try{const {instance}=await WebAssembly.instantiate(Buffer.concat(chunks),{lkjscript:{trap(c,l){throw Error('trap '+c+':'+l)}}});",
    );
    for (name, expected) in entries {
        script.push_str(&format!(
            "{{const actual=instance.exports['fn:{name}']();if(actual!=={expected}n)throw Error('{name}: expected {expected}, got '+actual);}}"
        ));
    }
    script.push_str("}catch(e){console.error(e);process.exitCode=1}})");
    let mut process = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process.stdin.take().unwrap().write_all(artifact).unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn rust_oracle(source: &str, entries: &[(&str, i64)]) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "lkjscript-method-contracts-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let input = directory.join("probe.rs");
    let executable = directory.join("probe");
    let mut full_source = format!("{source}\nfn main(){{");
    for (name, expected) in entries {
        full_source.push_str(&format!("assert_eq!({name}(),{expected});"));
    }
    full_source.push('}');
    std::fs::write(&input, full_source).unwrap();
    let compiled = Command::new("rustc")
        .arg("--edition=2024")
        .arg(&input)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let result = Command::new(&executable).output().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn builtin_and_declared_bounds_with_the_same_method_are_ambiguous() {
    for source in [
        "trait Other{fn clone(&self)->Self;} fn ambiguous<T:Clone+Other>(value:&T)->T{value.clone()}",
        "trait Other{fn clone(&self)->Self;} fn ambiguous<T:Copy+Other>(value:&T)->T{value.clone()}",
        "trait Other{fn clone_from(&mut self,other:&Self);} fn ambiguous<T:Clone+Other>(left:&mut T,right:&T){left.clone_from(right);}",
        "trait Other{fn eq(&self,other:&Self)->bool;} fn ambiguous<T:PartialEq+Other>(left:&T,right:&T)->bool{left.eq(right)}",
        "trait Other{fn ne(&self,other:&Self)->bool;} fn ambiguous<T:Eq+Other>(left:&T,right:&T)->bool{left.ne(right)}",
    ] {
        let error = check(source)
            .err()
            .expect("ambiguous unused generic method must fail");
        assert_eq!(error.code, "E_TRAIT", "{source}\n{error:?}");
        assert!(error.message.contains("ambiguous"), "{error:?}");
    }
}

#[test]
fn checked_builtin_and_declared_trait_calls_keep_their_selection() {
    let source = r#"
struct Item { value:i64 }
impl Clone for Item {fn clone(&self)->Self{Item{value:1}}}
trait Score {fn score(&self)->i64;}
impl Score for Item {fn score(&self)->i64{3}}
impl Item {
    fn clone(&self)->Self{Item{value:2}}
    fn score(&self)->i64{4}
}
fn copied<T:Clone>(value:&T)->T{value.clone()}
fn scored<T:Score>(value:&T)->i64{value.score()}
pub fn selected_clone()->i64{copied(&Item{value:0}).value}
pub fn direct_clone()->i64{Item{value:0}.clone().value}
pub fn selected_score()->i64{scored(&Item{value:0})}
pub fn direct_score()->i64{Item{value:0}.score()}
"#;
    let checked = check(source).unwrap();
    run(
        &wasm::emit(&checked).unwrap(),
        &[
            ("selected_clone", 1),
            ("direct_clone", 2),
            ("selected_score", 3),
            ("direct_score", 4),
        ],
    );
    rust_oracle(
        source,
        &[
            ("selected_clone", 1),
            ("direct_clone", 2),
            ("selected_score", 3),
            ("direct_score", 4),
        ],
    );
}

#[test]
fn checked_receiver_dereferences_do_not_change_when_a_parameter_becomes_a_reference() {
    let source = r#"
fn copied<T:Clone>(value:&T)->T{value.clone()}
trait Score {fn score(&self)->i64;}
impl Score for &i64 {fn score(&self)->i64{**self+1}}
fn scored<T:Score>(value:&T)->i64{value.score()}
pub fn cloned_reference()->i64 {
    let value:i64=42;let reference=&value;
    let copied_reference=copied::<&i64>(&reference);
    *copied_reference
}
pub fn scored_reference()->i64 {
    let value:i64=42;let reference=&value;
    scored::<&i64>(&reference)
}
"#;
    let checked = check(source).unwrap();
    run(
        &wasm::emit(&checked).unwrap(),
        &[("cloned_reference", 42), ("scored_reference", 43)],
    );
    rust_oracle(
        source,
        &[("cloned_reference", 42), ("scored_reference", 43)],
    );
}

#[test]
fn known_receiver_keeps_the_source_method_selected_under_generic_parameters() {
    let source = r#"
struct Wrapper<T>{value:T}
trait Score {fn score(&self)->i64;}
impl<T> Score for Wrapper<T> {fn score(&self)->i64{1}}
impl Wrapper<i64> {fn score(&self)->i64{2}}
fn scored<T>(value:&Wrapper<T>)->i64{value.score()}
pub fn selected()->i64{scored(&Wrapper{value:0i64})}
pub fn direct()->i64{Wrapper{value:0i64}.score()}
"#;
    let checked = check(source).unwrap();
    run(
        &wasm::emit(&checked).unwrap(),
        &[("selected", 1), ("direct", 2)],
    );
    rust_oracle(source, &[("selected", 1), ("direct", 2)]);
}

#[test]
fn concrete_specialization_executes_a_derived_clone() {
    let source = "#[derive(Clone)] struct Item{value:Box<i64>} fn copied<T:Clone>(value:&T)->T{value.clone()} pub fn probe()->i64{let value=Item{value:Box::new(42)};let copy=copied(&value);drop(value);*copy.value}";
    let checked = check(source).unwrap();
    run(&wasm::emit(&checked).unwrap(), &[("probe", 42)]);
    rust_oracle(source, &[("probe", 42)]);
}

#[test]
fn builtin_default_operations_have_checked_executable_bodies() {
    let source = r#"
struct Item(i64);
impl Clone for Item{fn clone(&self)->Self{Item(self.0)}}
impl PartialEq for Item{fn eq(&self,other:&Self)->bool{self.0==other.0}}
fn update<T:Clone>(left:&mut T,right:&T){left.clone_from(right);}
fn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}
fn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}
pub fn updated()->i64{let mut left=Item(1);let right=Item(42);update(&mut left,&right);left.0}
pub fn defaults()->i64{if different(&Item(1),&Item(2)) && same(&42i64,&42i64){42}else{0}}
"#;
    let checked = check(source).unwrap();
    run(
        &wasm::emit(&checked).unwrap(),
        &[("updated", 42), ("defaults", 42)],
    );
    rust_oracle(source, &[("updated", 42), ("defaults", 42)]);
}

#[test]
fn verification_types_and_functions_do_not_enter_a_generic_string_artifact() {
    let root = r#"
fn copied<T:Clone>(value:&T)->T{value.clone()}
fn scalar_copy<T:Copy>(value:&T)->T{value.clone()}
fn update<T:Clone>(target:&mut T,source:&T){target.clone_from(source);}
fn same<T:Eq>(left:&T,right:&T)->bool{left.eq(right)}
fn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}
pub fn probe()->i64 {
    let original=String::from_str("hé😀");
    let mut result=copied(&original);
    let other=String::from_str("other");
    update(&mut result,&other);
    if same(&result,&other) && different(&original,&result) {scalar_copy(&42i64)} else {0}
}
"#;
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "builtin_contracts.rs".to_string(),
        text: root.to_string(),
    });
    let bundle = SourceBundle {
        root: "builtin_contracts.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    };
    let checked = compiler::check_bundle(&bundle).unwrap();
    for name in checked.program().types.iter().map(|ty| &ty.name).chain(
        checked
            .program()
            .functions
            .iter()
            .map(|function| &function.name),
    ) {
        assert!(
            !name.contains("$rigid") && !name.contains("$contract"),
            "{name}"
        );
    }
    let artifact = wasm::emit(&checked).unwrap();
    for marker in [b"$rigid".as_slice(), b"$contract".as_slice()] {
        assert!(
            !artifact
                .windows(marker.len())
                .any(|window| window == marker)
        );
    }
    run(&artifact, &[("probe", 42)]);
}

// A cyclic K reference type exercises the projection representation without
// introducing a source alias or a numeric count that expands during checking.
fn cyclic_reference_program(depth: usize) -> lkjscript::k::Program {
    use lkjscript::k;
    let mut types = k::primitives();
    let reference = types.len();
    types.push(k::Type {
        name: "CyclicReference".to_string(),
        kind: k::TypeKind::Ref {
            inner: reference,
            mutable: false,
        },
    });
    let projections = vec![k::Projection::Deref; depth];
    let function = k::Function {
        file: 0,
        name: "cyclic".to_string(),
        params: 1,
        result: reference,
        return_source: Some(k::ReturnSource::Parameter {
            index: 0,
            projections: projections.clone(),
        }),
        locals: vec![
            k::Local {
                name: "parameter".to_string(),
                ty: reference,
                mutable: false,
            },
            k::Local {
                name: "returned".to_string(),
                ty: reference,
                mutable: false,
            },
        ],
        body: vec![
            k::Instruction {
                line: 1,
                op: k::Op::Read {
                    dst: 1,
                    place: k::Place {
                        local: 0,
                        projections,
                    },
                    moving: false,
                },
            },
            k::Instruction {
                line: 2,
                op: k::Op::Drop {
                    place: k::Place::local(0),
                },
            },
            k::Instruction {
                line: 3,
                op: k::Op::Return { src: 1 },
            },
        ],
        test: false,
    };
    k::Program {
        types,
        functions: vec![function],
        destructors: Vec::new(),
    }
}

#[test]
fn explicit_return_paths_check_and_round_trip_on_cyclic_reference_types() {
    for depth in [1, 2, 64] {
        let original = cyclic_reference_program(depth);
        lkjscript::check::check(original.clone()).unwrap();
        let text = lkjscript::k_text::write(&original);
        let header = text.lines().find(|line| line.starts_with("fn ")).unwrap();
        assert_eq!(header.matches(" deref").count(), depth);
        let parsed = lkjscript::k_text::read(&text).unwrap();
        assert!(
            parsed == original,
            "return projection path changed during text roundtrip"
        );
        lkjscript::check::check(parsed).unwrap();
    }
}

#[test]
fn return_path_text_rejects_encoded_numeric_repetition() {
    let text = lkjscript::k_text::write(&cyclic_reference_program(1));
    for count in [0, 1, 64, usize::MAX] {
        let old_form = text.replace(" from 0 deref", &format!(" from 0 deref {count}"));
        assert_ne!(old_form, text);
        let error = lkjscript::k_text::read(&old_form)
            .err()
            .expect("old numeric repetition must not expand a projection path");
        assert_eq!(error.code, "E_K_TEXT", "{error:?}");
    }
}

#[test]
fn return_contract_rejects_unsupported_projection_kinds_in_the_k_checker() {
    use lkjscript::k::{Projection, ReturnSource};
    for projection in [
        Projection::Field(0),
        Projection::VariantField(0, 0),
        Projection::Index(usize::MAX),
    ] {
        let mut original = cyclic_reference_program(1);
        original.functions[0].return_source = Some(ReturnSource::Parameter {
            index: 0,
            projections: vec![projection],
        });
        // The generic K place syntax preserves the requested projection. Its
        // return-contract legality is checked at the semantic K boundary.
        let parsed = lkjscript::k_text::read(&lkjscript::k_text::write(&original)).unwrap();
        assert!(parsed == original);
        let error = lkjscript::check::check(parsed)
            .err()
            .expect("unsupported return projection must reach and fail the K checker");
        assert_eq!(error.code, "E_BORROW", "{error:?}");
        assert!(error.message.contains("source contract"), "{error:?}");
    }
}
