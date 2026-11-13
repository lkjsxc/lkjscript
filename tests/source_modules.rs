#[path = "../bridge/library.rs"]
mod bootstrap_library;
#[path = "support/native.rs"]
mod native;
use lkjscript::{ast::Diagnostic, check, elaborate, k, source};

fn program(files: &[(&str, &str)]) -> Result<k::CheckedProgram, Diagnostic> {
    let mut inputs = bootstrap_library::standard_library();
    inputs.extend(files.iter().map(|(path, text)| source::SourceInput {
        path: path.to_string(),
        text: text.to_string(),
    }));
    let bundle = source::SourceBundle {
        library: Some("$std/lib.rs".to_string()),
        root: "main.rs".to_string(),
        files: inputs,
    };
    let parsed = source::parse_bundle(&bundle)?;
    check::check(elaborate::elaborate(&parsed)?)
}
fn run(files: &[(&str, &str)], expected: i64) {
    native::assert_values(&program(files).unwrap(), &[("probe", expected)]);
}
fn rejects(files: &[(&str, &str)], code: &str, text: &str) -> Diagnostic {
    let error = match program(files) {
        Ok(_) => panic!("expected rejection"),
        Err(e) => e,
    };
    assert_eq!(error.code, code, "{error:?}");
    assert!(error.message.contains(text), "{error:?}");
    error
}
#[test]
fn multi_file_lexical_lookup_aliases_and_trait_identity() {
    run(
        &[
            (
                "main.rs",
                "mod a; mod b; mod contract; use crate::a::Token as Alias; use crate::contract::Read; fn probe()->i64 {let t=Alias { value:10 }; b::choose()+t.read() }",
            ),
            (
                "a.rs",
                "use crate::contract::Read; pub struct Token {pub value:i64} impl Read for Token {fn read(&self)->i64{self.value}} fn pick()->i64{1}",
            ),
            (
                "b.rs",
                "fn pick()->i64{2} pub fn choose()->i64{pick()} pub struct Token {pub value:bool}",
            ),
            ("contract.rs", "pub trait Read {fn read(&self)->i64;}"),
        ],
        12,
    );
}
#[test]
fn sibling_nominals_and_aliases_keep_declaration_identity() {
    rejects(
        &[(
            "main.rs",
            "mod a{pub struct Token{pub value:i64}}mod b{pub struct Token{pub value:i64}} fn take(x:a::Token)->i64{x.value} fn probe()->i64{take(b::Token{value:3})}",
        )],
        "E_TYPE",
        "different declaration",
    );
    run(
        &[(
            "main.rs",
            "mod a{pub struct Token{pub value:i64}}use a::Token as Alias;fn take(x:a::Token)->i64{x.value}fn probe()->i64{take(Alias{value:3})}",
        )],
        3,
    );
}
#[test]
fn sibling_traits_with_same_spelling_do_not_overlap() {
    run(
        &[(
            "main.rs",
            "mod a{pub trait Read{fn a(&self)->i64;}}mod b{pub trait Read{fn b(&self)->i64;}}impl a::Read for i64{fn a(&self)->i64{*self+1}}impl b::Read for i64{fn b(&self)->i64{*self+2}}use a::Read as A;use b::Read as B;fn probe()->i64{let x:i64=10;x.a()+x.b()}",
        )],
        23,
    );
}
#[test]
fn private_construction_projection_and_methods_are_independent() {
    rejects(
        &[(
            "main.rs",
            "mod secret{pub struct Token{value:i64}}fn probe()->i64{let token=secret::Token{value:9};0}",
        )],
        "E_PRIVACY",
        "field secret::Token::value",
    );
    rejects(
        &[(
            "main.rs",
            "mod secret{pub struct Token{value:i64}pub fn token()->Token{Token{value:9}}}fn probe()->i64{let token=secret::token();token.value}",
        )],
        "E_PRIVACY",
        "field secret::Token::value",
    );
    rejects(
        &[(
            "main.rs",
            "mod secret{pub struct Token(pub i64);impl Token{fn read(&self)->i64{self.0}}pub fn token()->Token{Token(9)}}fn probe()->i64{secret::token().read()}",
        )],
        "E_PRIVACY",
        "method read",
    );
}
#[test]
fn private_items_cannot_be_imported_or_reexported() {
    rejects(
        &[(
            "main.rs",
            "mod secret{struct Token{value:i64}}use secret::Token;fn probe()->i64{0}",
        )],
        "E_PRIVACY",
        "secret::Token",
    );
    rejects(
        &[(
            "main.rs",
            "mod secret{struct Token{value:i64}pub use self::Token as Public;}fn probe()->i64{0}",
        )],
        "E_PRIVACY",
        "re-export",
    );
}
#[test]
fn restricted_visibility_and_nested_module_paths() {
    run(
        &[
            ("main.rs", "mod outer;fn probe()->i64{outer::choose()}"),
            (
                "outer/mod.rs",
                "mod inner;pub fn choose()->i64{self::inner::pick()+crate::outer::inside()}pub(crate) fn inside()->i64{3}",
            ),
            (
                "outer/inner.rs",
                "pub(super) fn pick()->i64{super::inside()+2}",
            ),
        ],
        8,
    );
    rejects(
        &[(
            "main.rs",
            "mod outer{pub mod inner{pub(super) fn pick()->i64{3}}}fn probe()->i64{outer::inner::pick()}",
        )],
        "E_PRIVACY",
        "outer::inner::pick",
    );
}
#[test]
fn local_option_shadows_the_prelude_without_replacing_it() {
    run(
        &[(
            "main.rs",
            "enum Option<T>{None,Some(T)}fn probe()->i64{let x:Option<i64>=Option::Some(3);let y:std::Option<i64>=std::Option::Some(4);let value=y.unwrap();match x{Option::Some(v)=>v+value,Option::None=>0}}",
        )],
        7,
    );
}
#[test]
fn import_aliases_for_variants_and_forward_globs() {
    run(
        &[(
            "main.rs",
            "use b::*;mod a{pub fn value()->i64{5}}mod b{pub use crate::a::*;}enum Choice{None,Some(i64)}use Choice::Some as Present;fn probe()->i64{let v=Present(value());match v{Choice::Some(x)=>x,Choice::None=>0}}",
        )],
        5,
    );
}
#[test]
fn distinct_inherent_methods_have_distinct_emitted_symbols() {
    run(
        &[(
            "main.rs",
            "struct A(i64);struct B(i64);impl A{fn read(&self)->i64{self.0}}impl B{fn read(&self)->i64{self.0+1}}fn probe()->i64{let a=A(2);let b=B(3);a.read()+b.read()}",
        )],
        6,
    );
}
#[test]
fn source_library_has_private_representation_and_real_imports() {
    run(
        &[(
            "main.rs",
            "use std::collections::BTreeMap;fn probe()->i64{let mut map:BTreeMap<i64,i64>=BTreeMap::new();map.insert(3,9);let text=String::from_utf8(vec![65u8]).unwrap();text.len() as i64+*map.get(&3).unwrap()}",
        )],
        10,
    );
    rejects(
        &[(
            "main.rs",
            "fn probe()->i64{let text=String::from_utf8(vec![65u8]).unwrap();text.bytes.len() as i64}",
        )],
        "E_PRIVACY",
        "bytes",
    );
}
#[test]
fn missing_and_ambiguous_files_are_named_module_errors() {
    rejects(
        &[("main.rs", "mod missing;fn probe()->i64{0}")],
        "E_MODULE_MISSING",
        "missing.rs",
    );
    rejects(
        &[
            ("main.rs", "mod child;fn probe()->i64{0}"),
            ("child.rs", ""),
            ("child/mod.rs", ""),
        ],
        "E_MODULE_AMBIGUOUS",
        "child/mod.rs",
    );
}
#[test]
fn child_file_semantic_locations_retain_original_identity() {
    let files = [
        ("main.rs", "mod child;fn probe()->i64{child::probe()}"),
        ("child.rs", "pub fn probe()->i64{true}"),
    ];
    let error = rejects(&files, "E_TYPE", "function result type differs");
    assert_ne!(error.file, 0);
}
#[test]
fn structural_type_text_scales_linearly_for_nested_boxes() {
    fn size(depth: usize) -> usize {
        let ty = format!("{}i64{}", "Box<".repeat(depth), ">".repeat(depth));
        let text = format!("fn consume(_x:{ty}){{}}fn probe()->i64{{0}}");
        let checked = program(&[("main.rs", &text)]).unwrap();
        lkjscript::k_text::write(checked.program()).len()
    }
    let small = size(32);
    let large = size(64);
    assert!(
        large < small * 3,
        "type text expanded superlinearly: {small} -> {large}"
    );
}

#[test]
fn child_file_checker_errors_retain_original_identity() {
    let files = [
        ("main.rs", "mod child;fn probe()->i64{child::probe()}"),
        (
            "child.rs",
            "pub fn probe()->i64{let x:i64=1;let view=&x;let second=&mut x;*view}",
        ),
    ];
    let error = rejects(&files, "E_BORROW", "immutable");
    assert_ne!(error.file, 0);
}
#[test]
fn duplicate_source_paths_are_rejected() {
    let error = rejects(
        &[
            ("main.rs", "fn probe()->i64{1}"),
            ("main.rs", "fn probe()->i64{2}"),
        ],
        "E_SOURCE_DUPLICATE",
        "main.rs",
    );
    assert_eq!(error.path, "main.rs");
}
#[test]
fn compiler_bundle_never_substitutes_hidden_library_bytes() {
    let mut files = bootstrap_library::standard_library();
    let library = files
        .iter_mut()
        .find(|f| f.path == "$std/bytes.rs")
        .unwrap();
    library.text = "pub fn value()->i64{27}".to_string();
    files.push(source::SourceInput {
        path: "main.rs".to_string(),
        text: "fn probe()->i64{std::bytes::value()}".to_string(),
    });
    let bundle = source::SourceBundle {
        root: "main.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    };
    let error = source::parse_bundle(&bundle).and_then(|p| elaborate::elaborate(&p));
    // The supplied bytes intentionally omit valid_utf8 required by string.rs.
    // Resolving that import must fail; substituting embedded bytes would hide it.
    assert_eq!(
        error
            .err()
            .expect("supplied library missing required byte helpers must fail resolution")
            .code,
        "E_IMPORT"
    );
}

#[test]
fn declarations_shadow_intrinsics_without_changing_identity() {
    run(
        &[(
            "main.rs",
            "struct Vec {value:i64}impl Vec{fn new(value:i64)->Vec{Vec{value}}}struct Box{value:i64}impl Box{fn new(value:i64)->Box{Box{value}}}fn drop(value:i64)->i64{value+10}mod std{pub mod mem{pub fn replace(left:i64,right:i64)->i64{left+right}}}fn probe()->i64{let a=Vec::new(2);let b=Box::new(3);drop(a.value)+std::mem::replace(b.value,4)}",
        )],
        19,
    );
}

#[test]
fn collection_results_keep_the_library_option_under_shadowing() {
    run(
        &[(
            "main.rs",
            "enum Option<T>{None,Some(T)}fn probe()->i64{let mut values:Vec<i64>=vec![9];let result:std::Option<i64>=values.pop();result.unwrap()}",
        )],
        9,
    );
}

#[test]
fn trait_methods_are_selected_from_resolved_lexical_scope() {
    run(
        &[(
            "main.rs",
            "mod a{pub trait Pick{fn pick(&self)->i64;}impl Pick for i64{fn pick(&self)->i64{1}}}mod b{pub trait Pick{fn pick(&self)->i64;}impl Pick for i64{fn pick(&self)->i64{2}}}use b::Pick as Selected;fn probe()->i64{let x:i64=0;x.pick()}",
        )],
        2,
    );
    rejects(
        &[(
            "main.rs",
            "mod a{pub trait Pick{fn pick(&self)->i64;}impl Pick for i64{fn pick(&self)->i64{1}}}fn probe()->i64{let x:i64=0;x.pick()}",
        )],
        "E_TRAIT",
        "not in scope",
    );
    rejects(
        &[(
            "main.rs",
            "mod a{trait Pick{fn pick(&self)->i64;}impl Pick for i64{fn pick(&self)->i64{1}}}fn probe()->i64{let x:i64=0;x.pick()}",
        )],
        "E_PRIVACY",
        "private trait",
    );
}
#[test]
fn declared_generic_bounds_make_trait_methods_available() {
    run(
        &[(
            "main.rs",
            "mod a{pub trait Parent{fn pick(&self)->i64;}pub trait Child:Parent{}impl Parent for i64{fn pick(&self)->i64{7}}impl Child for i64{}}fn call<T:a::Child>(x:T)->i64{x.pick()}fn probe()->i64{call(0i64)}",
        )],
        7,
    );
}
