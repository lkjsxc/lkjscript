//! Executable equality follows the resolved builtin trait, preserving evaluation
//! order, exact operand identity, owner reuse and ordinary temporary cleanup.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
#[path = "support/native.rs"]
mod native;
use std::process::Command;

const SOURCE: &str = r#"
mod misleading {
    pub trait PartialEq { fn eq(&self, other: &Self) -> bool; }
}
struct Explicit { value: Box<i64> }
impl Explicit {
    fn eq(&self, _other: &Self) -> bool { true }
}
impl misleading::PartialEq for Explicit {
    fn eq(&self, _other: &Self) -> bool { true }
}
impl PartialEq for Explicit {
    fn eq(&self, other: &Self) -> bool { *self.value == *other.value }
    fn ne(&self, _other: &Self) -> bool { false }
}
pub fn selected_methods() -> i64 {
    let left = Explicit { value: Box::new(1) };
    let right = Explicit { value: Box::new(2) };
    if left == right { return 1; };
    if left != right { return 2; };
    if !left.eq(&right) { return 3; };
    *left.value + *right.value + 39
}
struct Defaulted { value: Box<i64> }
impl Defaulted {
    fn ne(&self, _other: &Self) -> bool { false }
}
impl PartialEq for Defaulted {
    fn eq(&self, other: &Self) -> bool { *self.value == *other.value }
}
impl Clone for Defaulted {
    fn clone(&self) -> Self { Defaulted { value: Box::new(*self.value) } }
    fn clone_from(&mut self, other: &Self) { *self.value = *other.value; }
}
fn compare_then_clone<T: PartialEq + Clone>(left: &mut T, right: &T) -> bool {
    *left == *right && { left.clone_from(right); true }
}
pub fn generic_comparison_releases_its_loans() -> i64 {
    let mut left = Defaulted { value: Box::new(42) };
    let right = Defaulted { value: Box::new(42) };
    if compare_then_clone(&mut left, &right) { *left.value } else { 0 }
}
pub fn default_ne() -> i64 {
    let left = Defaulted { value: Box::new(20) };
    let right = Defaulted { value: Box::new(22) };
    if !(left != right) { return 1; };
    if left.ne(&right) { return 2; };
    *left.value + *right.value
}
fn make(order: &mut i64, value: i64) -> Defaulted {
    *order = *order * 10 + value;
    Defaulted { value: Box::new(value) }
}
pub fn operand_calls_once_in_order() -> i64 {
    let mut order: i64 = 0;
    if make(&mut order, 1) == make(&mut order, 2) { return 0; };
    order
}
pub fn scalar_left_read_precedes_rhs_effects() -> i64 {
    let mut value: i64 = 1;
    let equal = value == { value = 2; 2 };
    if equal { 0 } else { value }
}
pub fn loans_end_before_next_boolean_operand() -> i64 {
    let mut left = Defaulted { value: Box::new(1) };
    let right = Defaulted { value: Box::new(1) };
    if left == right && { *left.value = 20; true } {
        if &left != &right && { *left.value = 42; true } { return *left.value; };
    }
    0
}
struct Pair { left: Defaulted, right: Defaulted }
fn borrowed(left: &Defaulted, right: &Defaulted) -> bool { left == right }
fn nested_borrowed(left: &&Defaulted, right: &&Defaulted) -> bool { left == right }
fn exclusive(left: &mut Defaulted, right: &mut Defaulted) -> bool {
    let equal = left == right;
    *left.value = 20;
    *right.value = 22;
    equal
}
pub fn projections_dereferences_and_references() -> i64 {
    let mut pair = Pair {
        left: Defaulted { value: Box::new(1) },
        right: Defaulted { value: Box::new(1) }
    };
    if !(pair.left == pair.right) { return 1; };
    if !borrowed(&pair.left, &pair.right) { return 2; };
    if !nested_borrowed(&&pair.left, &&pair.right) { return 3; };
    if !exclusive(&mut pair.left, &mut pair.right) { return 4; };
    *pair.left.value + *pair.right.value
}
struct Tracked<'a> { drops: &'a mut i64, value: i64 }
impl<'a> Drop for Tracked<'a> { fn drop(&mut self) { *self.drops = *self.drops + 1; } }
impl<'a> PartialEq for Tracked<'a> {
    fn eq(&self, other: &Self) -> bool { self.value == other.value }
}
pub fn owners_drop_exactly_once() -> i64 {
    let mut left_drops: i64 = 0;
    let mut right_drops: i64 = 0;
    {
        let left = Tracked { drops: &mut left_drops, value: 7 };
        let right = Tracked { drops: &mut right_drops, value: 7 };
        if !(left == right) { return 0; };
        if left != right { return 0; };
    }
    left_drops * 10 + right_drops
}
pub fn temporary_owners_drop_exactly_once() -> i64 {
    let mut left_drops: i64 = 0;
    let mut right_drops: i64 = 0;
    let equal = Tracked { drops: &mut left_drops, value: 7 }
        == Tracked { drops: &mut right_drops, value: 7 };
    if equal { left_drops * 10 + right_drops } else { 0 }
}
struct Wrapper<T> { value: T }
impl<T: PartialEq> PartialEq for Wrapper<T> {
    fn eq(&self, other: &Self) -> bool { self.value == other.value }
}
fn identity<T>(value: T) -> T { value }
fn equal_results<T: PartialEq>(left: T, right: T) -> bool {
    identity(left) == identity(right)
}
pub fn generic_implementation_and_unknown_hints() -> i64 {
    let left = Wrapper { value: Defaulted { value: Box::new(1) } };
    let right = Wrapper { value: Defaulted { value: Box::new(1) } };
    if !(left == right) { return 1; };
    if equal_results(left, right) { 42 } else { 2 }
}
struct Factory {}
impl Factory { fn len(&self) -> Defaulted { Defaulted { value: Box::new(7) } } }
pub fn method_name_does_not_set_the_operand_type() -> i64 {
    if (Factory {}.len()) == (Factory {}.len()) { 42 } else { 0 }
}
enum Packet { Data { value: Box<i64> } }
impl PartialEq for Packet {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Packet::Data { value: left } => match other {
                Packet::Data { value: right } => **left == **right
            }
        }
    }
}
pub fn enum_equality() -> i64 {
    let left = Packet::Data { value: Box::new(7) };
    let right = Packet::Data { value: Box::new(7) };
    if left != right { return 0; };
    match left { Packet::Data { value } => *value + 35 }
}
mod nominal {
    pub struct Value { pub number: i64 }
    impl PartialEq for Value {
        fn eq(&self, other: &Self) -> bool { self.number == other.number }
    }
}
use nominal::Value as Imported;
type Alias = nominal::Value;
pub fn imported_alias_identity() -> i64 {
    let left = Imported { number: 7 };
    let right: Alias = Imported { number: 7 };
    if left == right { 42 } else { 0 }
}
pub fn scalar_inference_and_references() -> i64 {
    let left: i64 = -1;
    let right: i64 = -1;
    if 1 == 1u64 && -1i64 == -1 && &left == &right && true != false { 42 } else { 0 }
}
fn vector(order: &mut i64, value: i64) -> Vec<i64> {
    *order = *order * 10 + value;
    let mut result: Vec<i64> = Vec::new();
    result.push(value);
    result
}
pub fn vector_temporary_operands_are_evaluated_once() -> i64 {
    let mut order: i64 = 0;
    if vector(&mut order, 1) == vector(&mut order, 2) { return 0; };
    order
}
"#;

fn bundle(source: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "partial_eq.rs".to_string(),
        text: source.to_string(),
    });
    SourceBundle {
        root: "partial_eq.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn rejects(source: &str, code: &str, reason: &str) {
    match compiler::check_bundle(&bundle(source)) {
        Ok(_) => panic!("invalid equality was accepted: {source}"),
        Err(problem) => {
            assert_eq!(problem.code, code, "{problem:?}");
            assert!(problem.message.contains(reason), "{problem:?}");
            assert_eq!(problem.path, "partial_eq.rs");
        }
    }
}

#[test]
fn equality_matches_the_same_rust_source() {
    let probes = [
        "selected_methods",
        "default_ne",
        "generic_comparison_releases_its_loans",
        "operand_calls_once_in_order",
        "scalar_left_read_precedes_rhs_effects",
        "loans_end_before_next_boolean_operand",
        "projections_dereferences_and_references",
        "owners_drop_exactly_once",
        "temporary_owners_drop_exactly_once",
        "generic_implementation_and_unknown_hints",
        "method_name_does_not_set_the_operand_type",
        "enum_equality",
        "imported_alias_identity",
        "scalar_inference_and_references",
        "vector_temporary_operands_are_evaluated_once",
    ];
    let directory =
        std::env::temp_dir().join(format!("lkjscript-partial-eq-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut source = SOURCE.to_string();
    source.push_str("\nfn main(){\n");
    for name in probes {
        source.push_str(&format!("println!(\"{name} {{}}\",{name}());\n"));
    }
    source.push_str("}\n");
    std::fs::write(directory.join("main.rs"), source).unwrap();
    let binary = directory.join("oracle");
    let built = Command::new("rustc")
        .args([
            "--edition=2024",
            "--cap-lints=allow",
            "-C",
            "overflow-checks=yes",
        ])
        .arg(directory.join("main.rs"))
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let oracle = Command::new(&binary).output().unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let expected = String::from_utf8(oracle.stdout).unwrap();
    assert_eq!(expected.lines().count(), probes.len());
    native::assert_rows(&native::check(&bundle(SOURCE)), &expected);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn unrelated_traits_and_inherent_methods_do_not_grant_builtin_equality() {
    rejects(
        "mod other{pub trait PartialEq{fn eq(&self,other:&Self)->bool;}} struct Value{} impl other::PartialEq for Value{fn eq(&self,other:&Self)->bool{true}} fn bad(left:Value,right:Value)->bool{left==right}",
        "E_TRAIT",
        "builtin PartialEq",
    );
    rejects(
        "struct Value{} impl Value{fn eq(&self,other:&Self)->bool{true}} fn bad(left:Value,right:Value)->bool{left==right}",
        "E_TRAIT",
        "builtin PartialEq",
    );
}

#[test]
fn same_spelled_nominal_operands_remain_different_types() {
    rejects(
        "mod a{pub struct Value{pub number:i64} impl PartialEq for Value{fn eq(&self,other:&Self)->bool{self.number==other.number}}} mod b{pub struct Value{pub number:i64}} fn bad(left:a::Value,right:b::Value)->bool{left==right}",
        "E_TYPE",
        "operand types differ",
    );
}

#[test]
fn builtin_comparison_methods_must_have_the_exact_contract() {
    for method in [
        "fn eq(self,other:Self)->bool{true}",
        "fn eq(&mut self,other:&Self)->bool{true}",
        "fn eq(&self,other:Self)->bool{true}",
        "fn eq(&self,other:&Self)->i64{1}",
        "fn eq<T>(&self,other:&Self)->bool{true}",
    ] {
        rejects(
            &format!(
                "struct Value{{number:i64}} impl PartialEq for Value{{{method}}} fn bad(left:Value,right:Value)->bool{{left==right}}"
            ),
            "E_TRAIT",
            "contract",
        );
    }
    rejects(
        "struct Value{} impl PartialEq for Value{fn eq(&self,other:&Self)->bool{true} fn ne(&self,other:&Self)->i64{1}} fn bad(left:Value,right:Value)->bool{left!=right}",
        "E_TRAIT",
        "contract",
    );
    rejects(
        "struct Value{} impl PartialEq for Value{fn ne(&self,other:&Self)->bool{false}} fn bad(left:Value,right:Value)->bool{left==right}",
        "E_TRAIT",
        "missing a required method",
    );
}

#[test]
fn lhs_borrow_is_live_while_rhs_is_evaluated() {
    rejects(
        "struct Value{number:i64} impl PartialEq for Value{fn eq(&self,other:&Self)->bool{self.number==other.number}} fn change(value:&mut Value)->Value{value.number=2;Value{number:2}} fn bad()->bool{let mut value=Value{number:1};value==change(&mut value)}",
        "E_BORROW",
        "borrow",
    );
}

#[test]
fn derived_comparisons_execute_and_keep_owners_usable() {
    let source = "#[derive(PartialEq)] struct Value{number:i64} pub fn probe()->i64{let left=Value{number:21};let right=Value{number:21};if !(left==right)||left!=right{return 0;}left.number+right.number}";
    native::assert_bundle(&bundle(source), &[("probe", 42)]);
}
