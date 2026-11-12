//! A resolved function retains its owner independently of names and arena order.
use lkjscript::ast::{FunctionId, FunctionOwner, ImplId, TraitId};
use lkjscript::{resolve, syntax};

const SOURCE: &str = r#"
trait Value {
    fn get(&self) -> i64;
    fn defaulted(&self) -> i64 { 42 }
}
struct Item { value: i64 }
impl Item { fn get(&self) -> i64 { self.value } }
impl Value for Item { fn get(&self) -> i64 { self.value + 1 } }
fn get() -> i64 { 7 }
"#;

#[test]
fn resolved_owners_distinguish_free_inherent_and_trait_functions() {
    let source = resolve::resolve(&syntax::parse(SOURCE).unwrap()).unwrap();
    let owner = |index| source.function_owner(FunctionId { index });
    let declaration = FunctionOwner::Trait(TraitId { index: 0 });
    assert!(owner(source.traits[0].methods[0]) == declaration);
    assert!(owner(source.traits[0].methods[1]) == declaration);
    assert!(
        owner(source.impls[0].methods[0]) == FunctionOwner::Implementation(ImplId { index: 0 })
    );
    assert!(
        owner(source.impls[1].methods[0]) == FunctionOwner::Implementation(ImplId { index: 1 })
    );
    let free = source.functions.len() - 1;
    assert_eq!(source.functions[free].name, "get");
    assert!(owner(free) == FunctionOwner::Free);
    let retained = resolve::resolve(&source).unwrap();
    assert!(retained.function_owners == source.function_owners);
}

#[test]
fn ownership_indices_are_local_to_each_resolved_program() {
    let first = resolve::resolve(&syntax::parse(SOURCE).unwrap()).unwrap();
    let second = resolve::resolve(
        &syntax::parse("fn get()->i64{42} struct Item; impl Item{fn get()->i64{7}}").unwrap(),
    )
    .unwrap();
    assert!(first.function_owner(FunctionId { index: 0 }) != FunctionOwner::Free);
    assert!(second.function_owner(FunctionId { index: 0 }) == FunctionOwner::Free);
    assert!(
        second.function_owner(FunctionId { index: 1 })
            == FunctionOwner::Implementation(ImplId { index: 0 })
    );
}

#[test]
fn conflicting_or_missing_function_owners_are_diagnosed() {
    let mut duplicate = syntax::parse(SOURCE).unwrap();
    duplicate.impls[0]
        .methods
        .push(duplicate.traits[0].methods[0]);
    let error = resolve::resolve(&duplicate).expect_err("two owners accepted");
    assert_eq!(error.code, "E_NAME");
    assert!(error.message.contains("more than one method owner"));

    let mut missing = syntax::parse(SOURCE).unwrap();
    missing.impls[0].methods.push(missing.functions.len());
    let error = resolve::resolve(&missing).expect_err("missing function accepted");
    assert_eq!(error.code, "E_NAME");
    assert!(error.message.contains("unknown function"));
}

#[test]
fn free_recursive_calls_keep_identity_when_a_method_has_the_same_name() {
    let source = syntax::parse(
        "fn grow<T>(value:T){grow::<Vec<T>>(vec![value]);} struct Item; impl Item{fn grow()->i64{42}}",
    ).unwrap();
    let error = lkjscript::generics::validate(&source)
        .expect_err("same-spelled method hid the recursive free function");
    assert_eq!(error.code, "E_RECURSION");
}

#[test]
fn builtin_method_is_not_a_same_spelled_generic_free_call() {
    let source = syntax::parse(
        "fn len<T>(value:T)->usize{let values=vec![value];values.len()} fn probe()->usize{len(42i64)}",
    ).unwrap();
    lkjscript::generics::validate(&source).unwrap();
    let checked = lkjscript::elaborate::elaborate(&source).unwrap();
    lkjscript::check::check(checked).unwrap();
}
