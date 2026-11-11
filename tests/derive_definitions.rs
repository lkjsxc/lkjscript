//! Derive obligations are checked at their declaration, including unused generics.
use lkjscript::{ast::Diagnostic, compiler, source::SourceBundle, source::SourceInput};

fn checked(text: &str) -> Result<(), Diagnostic> {
    compiler::check_bundle(&SourceBundle {
        root: "derive_case.rs".to_string(),
        library: None,
        files: vec![SourceInput {
            path: "derive_case.rs".to_string(),
            text: text.to_string(),
        }],
    })
    .map(|_| ())
}

fn accepted(text: &str) {
    checked(text).unwrap_or_else(|error| panic!("valid derive rejected: {error:?}\n{text}"));
}

fn rejected(text: &str, code: &str, reason: &str) {
    let error = checked(text).expect_err("invalid unused derive was accepted");
    assert_eq!(error.code, code, "{error:?}\n{text}");
    assert!(error.message.contains(reason), "{error:?}\n{text}");
    assert_eq!(error.path, "derive_case.rs");
}

#[test]
fn unused_clone_requires_every_concrete_field() {
    for text in [
        "struct NoClone; #[derive(Clone)] struct Item {value:NoClone} pub fn probe()->i64{0}",
        "struct NoClone; #[derive(Clone)] enum Item {Empty, Value(NoClone)}",
        "struct NoClone; type Alias=NoClone; #[derive(Clone)] struct Item(Alias);",
        "struct NoClone; #[derive(Clone)] struct Item {value:Vec<Box<NoClone>>}",
        "struct NoClone; #[derive(Clone)] struct Item<T> {value:T, bad:NoClone}",
    ] {
        rejected(text, "E_TRAIT", "derive Clone requires field");
    }
}

#[test]
fn generic_derive_validation_uses_constraints_instead_of_sampling() {
    accepted("#[derive(Clone)] struct Item<T> {value:Vec<T>}");
    accepted("#[derive(Clone)] enum Chain<T> {End, Next(T,Box<Chain<T>>)}");
    accepted("#[derive(Copy,Clone)] struct Pair<T> {first:T,second:T}");
    accepted("#[derive(PartialEq,Eq,PartialOrd,Ord)] struct Key<T> {value:T}");
    rejected(
        "#[derive(Copy,Clone)] struct Impossible<T> {value:Vec<T>}",
        "E_TYPE",
        "derive Copy requires field",
    );
    rejected(
        "#[derive(Clone)] struct Impossible<'a,T> {value:&'a mut T}",
        "E_TRAIT",
        "derive Clone requires field",
    );
}

#[test]
fn a_later_instantiation_cannot_hide_an_invalid_derive() {
    rejected(
        "struct NoClone; #[derive(Clone)] struct Item<T> {value:T,bad:NoClone} fn probe()->i64{let x=Item{value:42i64,bad:NoClone};x.value}",
        "E_TRAIT",
        "derive Clone requires field bad",
    );
}

#[test]
fn shared_views_do_not_require_the_referents_to_clone() {
    accepted("struct NoClone; #[derive(Clone)] struct View<'a> {value:&'a NoClone}");
    rejected(
        "struct NoClone; #[derive(Clone)] struct View<'a> {value:&'a mut NoClone}",
        "E_TRAIT",
        "derive Clone requires field value",
    );
}

#[test]
fn equality_and_ordering_check_float_and_nested_field_requirements() {
    accepted("#[derive(PartialEq,PartialOrd)] struct Reading {value:f64}");
    rejected(
        "#[derive(PartialEq,Eq)] struct Reading {value:f64}",
        "E_TRAIT",
        "derive Eq requires field value",
    );
    rejected(
        "#[derive(PartialEq,PartialOrd,Eq,Ord)] enum Reading {Values(Vec<f64>)}",
        "E_TRAIT",
        "derive Eq requires field",
    );
}

#[test]
fn derived_supertraits_require_actual_evidence() {
    for (source, requirement) in [
        ("#[derive(Copy)] struct Marker;", "requires Clone"),
        ("#[derive(Eq)] struct Key(i64);", "requires PartialEq"),
        (
            "#[derive(PartialOrd)] struct Key(i64);",
            "requires PartialEq",
        ),
        ("#[derive(Ord)] struct Key(i64);", "requires Eq"),
    ] {
        rejected(source, "E_TRAIT", requirement);
    }
    accepted("#[derive(Copy,Clone)] struct Marker;");
    accepted("#[derive(PartialEq,Eq,PartialOrd,Ord)] enum Key {Empty,Number(i64)}");
    accepted(
        "#[derive(Copy)] struct Marker {value:i64} impl Clone for Marker {fn clone(&self)->Self {Marker{value:self.value}}}",
    );
}

#[test]
fn same_spelled_user_traits_are_not_builtin_derive_evidence() {
    rejected(
        "trait Clone{} struct Value; impl Clone for Value{} #[derive(Clone)] struct Holder(Value);",
        "E_TRAIT",
        "derive Clone requires field",
    );
    accepted(
        "mod a {#[derive(Clone)] pub struct Value;} mod b {pub struct Value;} #[derive(Clone)] struct Holder(a::Value);",
    );
    rejected(
        "mod a {#[derive(Clone)] pub struct Value;} mod b {pub struct Value;} #[derive(Clone)] struct Holder(b::Value);",
        "E_TRAIT",
        "derive Clone requires field",
    );
}

#[test]
fn duplicate_and_unsupported_derives_are_not_ignored() {
    rejected(
        "#[derive(Clone,Clone)] struct Item;",
        "E_TRAIT",
        "duplicate derive Clone",
    );
    rejected(
        "#[derive(Clone)] #[derive(Clone)] struct Item;",
        "E_TRAIT",
        "duplicate derive Clone",
    );
    for name in ["Debug", "Default", "Hash", "Unknown"] {
        rejected(
            &format!("#[derive({name})] struct Item;"),
            "E_UNSUPPORTED",
            &format!("derive {name}"),
        );
    }
}

#[test]
fn structural_derive_contracts_lower_affine_clone_bodies() {
    accepted(
        "#[derive(Clone)] struct Item {value:Box<i64>} fn probe()->i64{let item=Item{value:Box::new(42)};let cloned=item.clone();drop(item);*cloned.value}",
    );
}

#[test]
fn derived_and_explicit_implementations_cannot_overlap() {
    for source in [
        "#[derive(Clone)] struct Item {value:i64} impl Clone for Item {fn clone(&self)->Self{Item{value:self.value}}}",
        "#[derive(Clone)] struct Item<T> {value:T} impl Clone for Item<i64> {fn clone(&self)->Self{Item{value:self.value}}}",
        "#[derive(Clone)] struct Item {value:i64} type Alias=Item; impl Clone for Alias {fn clone(&self)->Self{Item{value:self.value}}}",
    ] {
        rejected(source, "E_TRAIT", "overlaps derive");
    }
    accepted(
        "trait Clone {fn clone(&self)->i64;} #[derive(Clone)] struct Item {value:i64} impl Clone for Item {fn clone(&self)->i64{self.value}}",
    );
}

#[test]
fn explicit_builtin_clone_evidence_has_its_actual_method_contract() {
    for (body, reason) in [
        ("", "missing a required method"),
        ("fn clone(&self)->i64{0}", "result type differs"),
        (
            "fn clone(&mut self)->Self{Item{value:0}}",
            "parameter type differs",
        ),
        (
            "fn clone<T>(&self)->Self{Item{value:0}}",
            "signature differs",
        ),
        (
            "fn other(&self)->Self{Item{value:0}}",
            "not declared by builtin",
        ),
    ] {
        rejected(
            &format!(
                "struct Item{{value:i64}} impl Clone for Item{{{body}}} #[derive(Clone)] struct Holder(Item);"
            ),
            "E_TRAIT",
            reason,
        );
    }
    accepted(
        "struct Item{value:i64} impl Clone for Item{fn clone(&self)->Self{Item{value:self.value}} fn clone_from(&mut self,source:&Self){self.value=source.value;}} #[derive(Clone)] struct Holder(Item);",
    );
}

#[test]
fn explicit_builtin_equality_evidence_has_its_actual_method_contract() {
    rejected(
        "struct Item{value:i64} impl Eq for Item{}",
        "E_TRAIT",
        "requires PartialEq",
    );
    rejected(
        "struct Item{value:i64} impl PartialEq for Item{fn eq(&self,other:&Self)->i64{0}}",
        "E_TRAIT",
        "result type differs",
    );
    rejected(
        "struct Item{value:i64} impl PartialEq for Item{fn eq(&self,other:Self)->bool{true}}",
        "E_TRAIT",
        "parameter type differs",
    );
    accepted(
        "struct Item{value:i64} impl PartialEq for Item{fn eq(&self,other:&Self)->bool{self.value==other.value}} impl Eq for Item{} #[derive(PartialEq,Eq)] struct Holder(Item);",
    );
}

#[test]
fn unimplemented_explicit_builtin_overrides_are_named() {
    for name in ["Copy", "PartialOrd", "Ord", "Data", "Send", "Sync"] {
        rejected(
            &format!("struct Item{{value:i64}} impl {name} for Item{{}}"),
            "E_UNSUPPORTED",
            &format!("explicit builtin {name}"),
        );
    }
}

#[test]
fn builtin_method_evidence_requires_a_self_receiver() {
    rejected(
        "struct Item{value:i64} impl Clone for Item{fn clone(value:&Self)->Self{Item{value:value.value}}} #[derive(Clone)] struct Holder(Item);",
        "E_TRAIT",
        "requires a self receiver",
    );
    rejected(
        "struct Item{value:i64} impl PartialEq for Item{fn eq(left:&Self,right:&Self)->bool{left.value==right.value}} #[derive(PartialEq)] struct Holder(Item);",
        "E_TRAIT",
        "requires a self receiver",
    );
}

#[test]
fn explicit_builtin_implementations_do_not_override_structural_proofs() {
    for source in [
        "impl Clone for i64{fn clone(&self)->Self{*self}}",
        "impl PartialEq for Vec<i64>{fn eq(&self,other:&Self)->bool{true}}",
        "type Values=Vec<i64>; impl Eq for Values{}",
    ] {
        rejected(
            source,
            "E_UNSUPPORTED",
            "requires a source nominal struct or enum",
        );
    }
    accepted(
        "struct Item{value:i64} type Alias=Item; impl Clone for Alias{fn clone(&self)->Self{Item{value:self.value}}}",
    );
}
