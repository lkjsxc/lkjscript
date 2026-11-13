//! Transparent type aliases preserve constructor identity, inference, and privacy.
use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
#[path = "support/native.rs"]
mod native;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn bundle(files: &[(&str, &str)]) -> SourceBundle {
    SourceBundle {
        root: "main.rs".to_string(),
        library: None,
        files: files
            .iter()
            .map(|(path, text)| SourceInput {
                path: path.to_string(),
                text: text.to_string(),
            })
            .collect(),
    }
}

fn directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lkjscript-alias-constructors-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn accepted(files: &[(&str, &str)], probes: &[(&str, i64)]) {
    let checked = native::check(&bundle(files));
    let directory = directory();
    for (path, source) in files {
        let path = directory.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    let root = directory.join("main.rs");
    let mut source = std::fs::read_to_string(&root).unwrap();
    source.push_str("\nfn main(){");
    for (name, expected) in probes {
        source.push_str(&format!("assert_eq!({name}(),{expected});"));
    }
    source.push('}');
    std::fs::write(&root, source).unwrap();
    let executable = directory.join("probe");
    let rust = Command::new("rustc")
        .args([
            "--edition=2024",
            "--cap-lints=allow",
            "-C",
            "overflow-checks=yes",
        ])
        .arg(&root)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        rust.status.success(),
        "{}",
        String::from_utf8_lossy(&rust.stderr)
    );
    let result = Command::new(&executable).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::remove_dir_all(directory).unwrap();

    native::assert_values(&checked, probes);
}

fn rejected(source: &str, code: &str, reason: &str, rust_reason: &str) {
    let error = compiler::check_bundle(&bundle(&[("main.rs", source)]))
        .err()
        .expect("invalid alias constructor must be rejected");
    assert_eq!(error.code, code, "{source}\n{error:?}");
    assert!(error.message.contains(reason), "{source}\n{error:?}");
    assert_eq!(error.path, "main.rs", "{error:?}");
    let directory = directory();
    let input = directory.join("main.rs");
    std::fs::write(&input, source).unwrap();
    let result = Command::new("rustc")
        .args([
            "--edition=2024",
            "--crate-type=lib",
            "--emit=metadata",
            "--cap-lints=allow",
        ])
        .arg(input)
        .arg("-o")
        .arg(directory.join("probe.rmeta"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    assert!(!result.status.success(), "Rust accepted: {source}");
    assert!(
        String::from_utf8_lossy(&result.stderr).contains(rust_reason),
        "{source}\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn alias_arguments_and_field_inference_match_rust() {
    let source = r#"
struct Thing {value:i64}
type Alias=Thing;
type Identity<T> =T;
struct Pair<A,B>{first:A,second:B}
type Swapped<T,U> =Pair<U,T>;
type Fixed<T> =Swapped<T,u8>;
type Nested<T> =Pair<Box<T>,u8>;
type Chain<X> =Nested<X>;
type TupleAlias<T> =Pair<(T,u8),u8>;
type ArrayAlias<T> =Pair<[T;2],u8>;
struct Holder<T>{value:T}
type Held<T> =Holder<T>;
type Borrowed<'a,T> =Holder<&'a T>;
struct Empty;
type EmptyAlias=Empty;
impl<T> Holder<T>{fn make(value:T)->Self{Self{value}}}
fn generic<T>(unused:T)->i64{let result=Held{value:42i64};result.value}
fn preserve<T>(value:T)->Holder<T>{Held{value}}
fn take_alias<T>(value:Held<T>)->T{value.value}
pub fn alias_argument_inference()->i64{take_alias(Held{value:42i64})}
pub fn concrete()->i64{Alias{value:42}.value}
pub fn identity_alias()->i64{Identity::<Thing>{value:42}.value}
pub fn explicit()->i64{let v=Swapped::<i64,u8>{second:40,first:2};v.second+v.first as i64}
pub fn inferred()->i64{let v=Swapped{first:2u8,second:40i64};v.second+v.first as i64}
pub fn partial_arguments()->i64{let v=Swapped::<_,u8>{second:40i64,first:2};v.second+v.first as i64}
pub fn fixed_argument()->i64{let v=Fixed{second:40i64,first:2};v.second+v.first as i64}
pub fn expected()->i64{let v:Fixed<i64> =Fixed{second:40,first:2};v.second+v.first as i64}
pub fn nested_chain()->i64{let v=Chain{first:Box::new(40i64),second:2};*v.first+v.second as i64}
pub fn tuple_argument()->i64{let v=TupleAlias{first:(40i64,1u8),second:1};let (a,b)=v.first;a+b as i64+v.second as i64}
pub fn array_argument()->i64{let v=ArrayAlias{first:[20i64,20i64],second:2};v.first[0]+v.first[1]+v.second as i64}
pub fn caller_parameter_is_not_a_constructor_parameter()->i64{generic(true)}
pub fn parameter_transport()->i64{preserve(42i64).value}
pub fn stored_reference()->i64{let number:i64=42;let v=Borrowed{value:&number};*v.value}
pub fn self_constructor()->i64{Holder::<i64>::make(42).value}
pub fn empty_alias()->i64{let value=EmptyAlias{};drop(value);42}
"#;
    accepted(
        &[("main.rs", source)],
        &[
            ("concrete", 42),
            ("identity_alias", 42),
            ("explicit", 42),
            ("inferred", 42),
            ("partial_arguments", 42),
            ("fixed_argument", 42),
            ("expected", 42),
            ("nested_chain", 42),
            ("tuple_argument", 42),
            ("array_argument", 42),
            ("caller_parameter_is_not_a_constructor_parameter", 42),
            ("parameter_transport", 42),
            ("alias_argument_inference", 42),
            ("stored_reference", 42),
            ("self_constructor", 42),
            ("empty_alias", 42),
        ],
    );
}

#[test]
fn enum_aliases_use_the_same_constructor_and_pattern_identity() {
    let source = r#"
enum Packet<T,U>{Data{payload:T,tag:U},Tuple(T,U),End}
type Alias<T> =Packet<Box<T>,u8>;
type Chain<X> =Alias<X>;
fn read(value:Packet<Box<i64>,u8>)->i64{
    match value{
        Chain::Data{payload,tag}=>*payload+tag as i64,
        Chain::Tuple(payload,tag)=>*payload+tag as i64,
        Chain::End=>42
    }
}
pub fn inferred()->i64{read(Chain::Data{tag:2,payload:Box::new(40i64)})}
pub fn explicit()->i64{read(Alias::<i64>::Data{payload:Box::new(40),tag:2})}
pub fn expected()->i64{let value:Alias<i64> =Alias::Data{tag:2,payload:Box::new(40)};read(value)}
pub fn tuple()->i64{read(Alias::Tuple(Box::new(40i64),2))}
pub fn unit()->i64{let value:Alias<i64> =Alias::End;read(value)}
"#;
    accepted(
        &[("main.rs", source)],
        &[
            ("inferred", 42),
            ("explicit", 42),
            ("expected", 42),
            ("tuple", 42),
            ("unit", 42),
        ],
    );
}

#[test]
fn aliases_cross_files_without_changing_nominal_identity() {
    accepted(
        &[
            (
                "main.rs",
                r#"
mod a;mod b;
use crate::a::Alias as Imported;
use crate::b::Thing as Other;
type Chain=Imported;
pub fn probe()->i64{
    let left=Chain{value:40};
    let right=Other{value:2};
    a::read(left)+b::read(right)
}
"#,
            ),
            (
                "a.rs",
                "pub struct Thing{pub value:i64} pub type Alias=Thing;pub fn read(value:Thing)->i64{value.value}",
            ),
            (
                "b.rs",
                "pub struct Thing{pub value:i64} pub fn read(value:Thing)->i64{value.value}",
            ),
        ],
        &[("probe", 42)],
    );
}

#[test]
fn field_evaluation_order_and_method_candidate_trials_are_preserved() {
    let source = r#"
struct Pair<A,B>{first:A,second:B}
type Alias=Pair<i64,i64>;
fn next(counter:&mut i64)->i64{*counter=*counter+1;*counter}
impl<T> Pair<T,T>{fn score(&self)->i64{1}}
impl Pair<i64,bool>{fn score(&self)->i64{42}}
pub fn order()->i64{let mut counter:i64=0;let value=Alias{second:next(&mut counter),first:next(&mut counter)};value.first*10+value.second+21}
pub fn disjoint_candidate()->i64{Pair{first:0i64,second:false}.score()}
struct Tuple(i64);
use Tuple as Imported;
pub fn value_import()->i64{Imported(42).0}
"#;
    accepted(
        &[("main.rs", source)],
        &[
            ("order", 42),
            ("disjoint_candidate", 42),
            ("value_import", 42),
        ],
    );
}

#[test]
fn invalid_alias_constructors_fail_for_the_intended_type_reason() {
    for (source, reason, rust_reason) in [
        (
            "struct Thing{value:i64} type Alias=Thing;fn probe(){let v=Alias{value:true};}",
            "type differs",
            "E0308",
        ),
        (
            "struct Pair<A,B>{first:A,second:B} type Same<T> =Pair<T,T>;fn probe(){let v=Same{first:1u8,second:true};}",
            "generic arguments disagree",
            "E0308",
        ),
        (
            "mod a{pub struct Thing{pub value:i64}pub type Alias=Thing;}mod b{pub struct Thing{pub value:i64}}fn probe(){let v:b::Thing=a::Alias{value:1};}",
            "different declaration",
            "E0308",
        ),
        (
            "struct Thing{value:i64}type Alias=Thing;fn probe(){let v=Alias::<i64>{value:1};}",
            "type arguments",
            "E0107",
        ),
        (
            "struct Thing{left:i64,right:i64}type Alias=Thing;fn probe(){let v=Alias{left:1};}",
            "missing fields",
            "E0063",
        ),
        (
            "struct Thing{value:i64}type Alias=Thing;fn probe(){let v=Alias{value:1,value:2};}",
            "duplicate field",
            "E0062",
        ),
        (
            "struct Thing{value:i64}type Alias=Thing;fn probe(){let v=Alias{other:1};}",
            "unknown field",
            "E0560",
        ),
        (
            "enum Packet<T>{Data{value:T}}type Alias<T> =Packet<T>;fn probe(){let v=Alias::Data{other:1i64};}",
            "unknown variant field",
            "E0559",
        ),
        (
            "type Alias=i64;fn probe(){let v=Alias{};}",
            "nominal struct or enum",
            "E0071",
        ),
        (
            "struct Holder<T>{value:T}type Alias<T> =Holder<T>;fn unused<U>(v:U){let bad=Alias::<i64>{value:true};}",
            "generic arguments disagree",
            "E0308",
        ),
    ] {
        rejected(source, "E_TYPE", reason, rust_reason);
    }
    rejected(
        "struct Tuple(i64);type Alias=Tuple;fn probe(){let v=Alias(42);}",
        "E_NAME",
        "unresolved name Alias",
        "E0423",
    );
}

#[test]
fn aliases_preserve_private_fields_and_private_underlying_types() {
    for (source, rust_reason) in [
        (
            "mod secret{pub struct Token{value:i64}pub type Alias=Token;}fn probe(){let value=secret::Alias{value:9};}",
            "E0451",
        ),
        (
            "mod secret{pub struct Token{value:i64}pub type Alias=Token;pub fn make()->Alias{Token{value:9}}}fn probe()->i64{secret::make().value}",
            "E0616",
        ),
        (
            "mod secret{struct Token{pub value:i64}pub type Alias=Token;}fn probe(){let value=secret::Alias{value:9};}",
            "private",
        ),
    ] {
        rejected(source, "E_PRIVACY", "private", rust_reason);
    }
}

#[test]
fn nested_partial_alias_expectation_propagates_before_literal_defaults() {
    // The alias fixes the nested u8 leaf while another leaf determines T.
    let source = r#"
struct Pair<A,B>{first:A,second:B}
struct Holder<T>{value:T}
type Alias<T> = Holder<Pair<T,u8>>;
pub fn probe()->i64 {
    let value=Alias{value:Pair{first:40i64,second:2}};
    value.value.first+value.value.second as i64
}
"#;
    accepted(&[("main.rs", source)], &[("probe", 42)]);

    let explicit = source.replace("let value=Alias{", "let value=Alias::<i64>{");
    let typed = source.replace("second:2}", "second:2u8}");
    assert_ne!(explicit, source);
    assert_ne!(typed, source);
    accepted(&[("main.rs", &explicit)], &[("probe", 42)]);
    accepted(&[("main.rs", &typed)], &[("probe", 42)]);
}
