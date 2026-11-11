//! Generic definitions are checked under rigid parameters before specialization.
use lkjscript::{check, elaborate, k, syntax, wasm};

fn compile(source: &str) -> Result<k::CheckedProgram, lkjscript::ast::Diagnostic> {
    let parsed = syntax::parse(source).expect("definition fixture must parse");
    check::check(elaborate::elaborate(&parsed)?)
}

fn accepted(source: &str) {
    if let Err(error) = compile(source) {
        panic!("valid definition rejected: {error:?}");
    }
}

fn rejected(source: &str, code: &str) {
    match compile(source) {
        Ok(_) => panic!("invalid uninstantiated definition was accepted"),
        Err(error) => assert_eq!(error.code, code, "{error:?}"),
    }
}

#[test]
fn unused_generic_arithmetic_is_checked() {
    rejected(
        "fn bad<T>(x:T)->i64 {1i64 + true} fn probe()->i64{0}",
        "E_TYPE",
    );
}

#[test]
fn a_successful_specialization_cannot_hide_a_bad_definition() {
    rejected(
        "fn bad<T>(x:T)->i64{x} fn probe()->i64{bad(42i64)}",
        "E_TYPE",
    );
}

#[test]
fn rigid_parameters_do_not_assume_concrete_structure() {
    rejected("fn bad<T>(x:T)->i64{x.value}", "E_TYPE");
    rejected("fn bad<T>(x:T)->i64{x as i64}", "E_TYPE");
}

#[test]
fn unconstrained_parameters_remain_affine() {
    rejected("fn bad<T>(x:T)->T {let moved=x; x}", "E_MOVE");
    accepted("fn identity<T>(x:T)->T{x}");
    accepted("fn duplicate<T:Copy>(x:T)->(T,T){(x,x)}");
}

#[test]
fn generic_local_borrows_cannot_escape() {
    rejected("fn bad<'a,T>(x:T)->&'a T {&x}", "E_BORROW");
}

#[test]
fn calls_require_the_declared_generic_bounds() {
    rejected("fn need<T:Copy>(x:T){} fn bad<U>(x:U){need(x);}", "E_TRAIT");
    accepted("fn need<T:Copy>(x:T){} fn good<U:Copy>(x:U){need(x);}");
}

#[test]
fn opaque_types_do_not_gain_automatic_traits() {
    for marker in ["Data", "Send", "Sync"] {
        rejected(
            &format!("fn need<T:{marker}>(x:T){{}} fn bad<U>(x:U){{need(x);}}"),
            "E_TRAIT",
        );
        accepted(&format!(
            "fn need<T:{marker}>(x:T){{}} fn good<U:{marker}>(x:U){{need(x);}}"
        ));
    }
    rejected(
        "fn need<T:Data>(x:T){} fn bad<U:Copy>(x:U){need(x);}",
        "E_TRAIT",
    );
}

#[test]
fn structural_obligations_reach_rigid_children() {
    rejected(
        "fn need<T:Send>(x:T){} fn bad<U>(x:Vec<U>){need(x);}",
        "E_TRAIT",
    );
    accepted("fn need<T:Send>(x:T){} fn good<U:Send>(x:Vec<U>){need(x);}");
}

#[test]
fn declared_trait_methods_use_their_signatures() {
    accepted("trait Score{fn score(&self)->i64;} fn good<T:Score>(x:&T)->i64{x.score()}");
    rejected(
        "trait Score{fn score(&self)->i64;} fn bad<T:Score>(x:&T)->bool{x.score()}",
        "E_TYPE",
    );
}

#[test]
fn a_concrete_implementation_is_not_an_arbitrary_parameter_witness() {
    rejected(
        "trait Score{fn score(&self)->i64;} impl Score for i64{fn score(&self)->i64{*self}} fn need<T:Score>(x:T){} fn bad<U>(x:U){need(x);}",
        "E_TRAIT",
    );
}

#[test]
fn comparisons_use_borrowed_declared_contracts() {
    accepted("fn less<T:Ord>(left:&T,right:&T)->bool{*left < *right}");
    rejected(
        "fn bad<T:Copy>(left:T,right:T)->bool{left < right}",
        "E_TRAIT",
    );
}

#[test]
fn supertrait_evidence_is_available_at_the_definition() {
    accepted(
        "trait Base{fn score(&self)->i64;} trait Child:Base{} fn good<T:Child>(x:&T)->i64{x.score()}",
    );
}

#[test]
fn associated_results_retain_their_declared_identity() {
    accepted(
        "trait Source{type Item;fn get(&self)->Self::Item;} fn good<T:Source>(x:&T)->T::Item{x.get()}",
    );
    rejected(
        "trait Source{type Item;fn get(&self)->Self::Item;} fn bad<T:Source>(x:&T)->i64{x.get()}",
        "E_TYPE",
    );
}

#[test]
fn method_type_parameters_remain_rigid_at_use_sites() {
    accepted("trait Mapper{fn take<U>(&self,x:U)->U;} fn good<T:Mapper,U>(x:&T,y:U)->U{x.take(y)}");
}

#[test]
fn generic_impl_methods_are_checked_even_when_unused() {
    rejected(
        "struct Holder<T>{value:T} impl<T> Holder<T>{fn bad(&self)->i64{true}}",
        "E_TYPE",
    );
}

#[test]
fn a_user_trait_named_copy_does_not_grant_builtin_copy() {
    rejected("trait Copy{} fn bad<T:Copy>(x:T)->(T,T){(x,x)}", "E_MOVE");
}

#[test]
fn unreachable_generic_expressions_still_have_to_type_check() {
    rejected("fn bad<T>(x:T)->i64{return 0; 1i64+true}", "E_TYPE");
}

#[test]
fn abstract_verification_cannot_leak_into_an_executable() {
    let checked=compile("trait Score{fn score(&self)->i64;} struct Ticket{value:i64} impl Score for Ticket{fn score(&self)->i64{self.value}} fn read<T:Score>(x:&T)->i64{x.score()} fn probe()->i64{read(&Ticket{value:42})}").unwrap();
    assert!(
        checked
            .program()
            .types
            .iter()
            .all(|ty| !ty.name.starts_with("$rigid:"))
    );
    assert!(
        checked
            .program()
            .functions
            .iter()
            .all(|function| !function.name.starts_with("$contract:"))
    );
    assert!(!wasm::emit(&checked).unwrap().is_empty());
}

#[test]
fn statically_false_branches_still_check_generic_bodies() {
    rejected(
        "fn bad<T>(x:T)->i64{if false{let wrong=1i64+true;}0}",
        "E_TYPE",
    );
}

#[test]
fn unused_trait_default_bodies_are_checked() {
    rejected("trait Score{fn score(&self)->i64{true}}", "E_TYPE");
}

#[test]
fn nominal_and_alias_type_arguments_require_their_declared_bounds() {
    rejected(
        "struct Holder<T:Copy>{value:T} fn bad<U>(x:U)->Holder<U>{Holder{value:x}}",
        "E_TRAIT",
    );
    rejected(
        "type Holder<T:Copy>=(T,); fn bad<U>(x:U)->Holder<U>{(x,)}",
        "E_TRAIT",
    );
    accepted("struct Holder<T:Copy>{value:T} fn good<U:Copy>(x:U)->Holder<U>{Holder{value:x}}");
}

#[test]
fn derived_copy_on_generic_layouts_is_conditional() {
    accepted(
        "#[derive(Copy,Clone)] struct Holder<T>{value:T} fn wrap<T>(x:T)->Holder<T>{Holder{value:x}}",
    );
    accepted(
        "#[derive(Copy,Clone)] enum Maybe<T>{None,Some(T)} fn wrap<T>(x:T)->Maybe<T>{Maybe::Some(x)}",
    );
}

#[test]
fn recursive_associated_bounds_are_introduced_lazily() {
    accepted(
        "trait Chain{type Next:Chain;fn next(self)->Self::Next;} fn next<T:Chain>(x:T)->T::Next{x.next()}",
    );
}

#[test]
fn user_traits_do_not_satisfy_same_spelled_builtin_predicates() {
    for marker in ["Copy", "Data", "Send", "Sync"] {
        rejected(
            &format!(
                "mod builtin{{pub fn need<T:{marker}>(x:T){{}}}} trait {marker}{{}} fn bad<U:{marker}>(x:U){{builtin::need(x);}}"
            ),
            "E_TRAIT",
        );
    }
}

#[test]
fn explicit_generic_method_arguments_are_honored() {
    rejected(
        "trait Mapper{fn take<U>(&self,x:U)->U;} fn bad<T:Mapper>(x:&T)->i64{x.take::<bool>(1i64)}",
        "E_TYPE",
    );
    rejected(
        "trait Mapper{fn take<U>(&self,x:U)->U;} struct M{} impl Mapper for M{fn take<U>(&self,x:U)->U{x}} fn probe()->i64{M{}.take::<bool>(1i64)}",
        "E_TYPE",
    );
    accepted(
        "trait Mapper{fn take<U>(&self,x:U)->U;} struct M{} impl Mapper for M{fn take<U>(&self,x:U)->U{x}} fn probe()->i64{M{}.take::<i64>(42)}",
    );
}

#[test]
fn checked_associated_contracts_lower_with_concrete_witnesses() {
    accepted(
        "trait Source{type Item;fn get(&self)->Self::Item;} struct Integer{value:i64} impl Source for Integer{type Item=i64;fn get(&self)->Self::Item{self.value}} fn read<T:Source>(x:&T)->T::Item{x.get()} fn probe()->i64{read(&Integer{value:42})}",
    );
}

#[test]
fn explicit_clone_methods_are_selected_by_declaration() {
    accepted(
        "struct Thing(i64); impl Thing{fn clone(&self)->i64{42}} fn probe()->i64{Thing(1).clone()}",
    );
    accepted(
        "#[derive(Copy,Clone)] struct Thing(i64); impl Thing{fn clone(&self)->Thing{Thing(42)}} fn probe()->i64{Thing(1).clone().0}",
    );
    accepted(
        "mod custom{pub trait Clone{fn clone(&self)->i64;}} fn read<T:custom::Clone>(x:&T)->i64{x.clone()}",
    );
}

#[test]
fn destructor_registration_uses_the_builtin_trait_identity() {
    for source in [
        "trait Drop{fn value(&self)->i64;} struct Thing{} impl Drop for Thing{fn value(&self)->i64{42}} fn probe()->i64{Thing{}.value()}",
        "trait Drop{fn drop(&self)->i64;} struct Thing{} impl Drop for Thing{fn drop(&self)->i64{42}} fn probe()->i64{Thing{}.drop()}",
        "struct Thing{} impl Thing{fn drop(&self)->i64{42}} fn probe()->i64{Thing{}.drop()}",
    ] {
        let checked = compile(source)
            .expect("ordinary same-spelled trait/method must not register a destructor");
        assert!(checked.program().destructors.is_empty());
    }
    accepted(
        "struct Thing{} impl Drop for Thing{fn drop(&mut self){}} impl Thing{fn drop(&self)->i64{42}} fn probe()->i64{Thing{}.drop()}",
    );
    rejected(
        "struct Thing{} impl Drop for Thing{fn drop(&mut self){}} fn probe(){let mut value=Thing{};value.drop();}",
        "E_TYPE",
    );
}
