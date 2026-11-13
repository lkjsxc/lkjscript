//! Written statement boundaries survive parsing and normal type checking.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::ast::{Expr, Stmt};
use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, syntax};
#[path = "support/native.rs"]
mod native;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const SOURCE: &str = r#"
pub fn if_before_dereference() -> i64 {
    let value: i64 = 42; let reference = &value;
    if true {} *reference
}
pub fn if_before_negative() -> i64 { if true {} -42i64 }
pub fn block_before_dereference() -> i64 {
    let value: i64 = 42; let reference = &value;
    {} *reference
}
pub fn block_before_group() -> i64 { {} (42i64) }
pub fn if_before_array() -> i64 { if true {} [42i64][0] }
pub fn if_before_borrow() -> i64 {
    let value: i64 = 42;
    let reference = { if true {} &value };
    *reference
}
pub fn if_before_not() -> i64 {
    let value = { if true {} !false };
    if value { 42 } else { 0 }
}
pub fn while_before_dereference() -> i64 {
    let value: i64 = 42; let reference = &value;
    while false {} *reference
}
pub fn loop_before_dereference() -> i64 {
    let value: i64 = 42; let reference = &value;
    loop { break; } *reference
}
pub fn for_before_dereference() -> i64 {
    let value: i64 = 42; let reference = &value;
    for index in 0..2 {} *reference
}
pub fn match_before_dereference() -> i64 {
    let value: i64 = 42; let reference = &value;
    match 0i64 { _ => {} } *reference
}
pub fn parenthesized_if_binary() -> i64 { (if true { 6i64 } else { 7i64 }) * 7i64 }
pub fn parenthesized_block_binary() -> i64 { (({ 6i64 })) * 7i64 }
pub fn parenthesized_block_cast() -> i64 { ({ 42i64 }) as i64 }
pub fn initializer_if_binary() -> i64 {
    let value = if true { 6i64 } else { 7i64 } * 7i64;
    value
}
pub fn return_if_binary() -> i64 { return if true { 6i64 } else { 7i64 } * 7i64; }
pub fn binary_right_block() -> i64 { 2i64 + { 40i64 } }
pub fn binary_right_if() -> i64 { 2i64 + if true { 40i64 } else { 39i64 } }
fn identity(value: i64) -> i64 { value }
pub fn argument_if_binary() -> i64 { identity(if true { 6i64 } else { 7i64 } * 7i64) }
pub fn prefixed_block_value() -> i64 { -{ 42i64 } }
pub fn block_method_continuation() -> i64 { { 6i64 }.clone() * 7i64 }
pub fn if_method_continuation() -> i64 { if true { 6i64 } else { 7i64 }.clone() * 7i64 }
struct Field { value: i64 }
pub fn if_field_continuation() -> i64 {
    if true { Field { value: 42 } } else { Field { value: 0 } }.value
}
pub fn parenthesized_index() -> i64 { (if true { [42i64] } else { [0i64] })[0] }
pub fn if_arm_before_negative_pattern() -> i64 {
    match 0i64 {
        0 => if true { 42i64 } else { 0i64 }
        -1 => 1i64,
        _ => 2i64,
    }
}
pub fn block_arm_method_continuation() -> i64 {
    match 0i64 { 0 => { 42i64 }.clone(), _ => 0i64 }
}
pub fn control_flow_arms_can_omit_commas() -> i64 {
    let mut value: i64 = 0;
    match 0i64 {
        0 => while value < 1 { value = value + 1; }
        1 => for index in 0..1 { value = value + 1; }
        _ => loop { break; }
    }
    value + 41
}
pub fn explicit_semicolons_discard_block_values() -> i64 {
    { 1i64 };
    if true { 2i64 } else { 3i64 };
    match 0i64 { _ => 4i64 };
    42
}
pub fn unparenthesized_block_tail_is_a_value() -> i64 { { 42i64 } }
pub fn parenthesized_block_tail_is_a_value() -> i64 { ({ 42i64 }) }
pub fn unparenthesized_if_tail_is_a_value() -> i64 { if true { 42 } else { 0 } }
pub fn return_still_exits_before_later_effects() -> i64 {
    let mut value: i64 = 0;
    return 42;
    value = 1;
    value
}
pub fn diverging_implicit_statement() -> i64 {
    if true { return 42; } else { return 0; }
    1
}
struct Tracked<'a> { count: &'a mut i64 }
impl<'a> Drop for Tracked<'a> { fn drop(&mut self) { *self.count = *self.count + 1; } }
pub fn discarded_affine_block_values_drop_once() -> i64 {
    let mut count: i64 = 0;
    { Tracked { count: &mut count } };
    if true { Tracked { count: &mut count } } else { Tracked { count: &mut count } };
    count * 21
}
"#;

fn bundle(source: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "block_statements.rs".to_string(),
        text: source.to_string(),
    });
    SourceBundle {
        root: "block_statements.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lkjscript-block-statements-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn statement_and_value_contexts_match_the_same_rust_source() {
    let probes = [
        "if_before_dereference",
        "if_before_negative",
        "block_before_dereference",
        "block_before_group",
        "if_before_array",
        "if_before_borrow",
        "if_before_not",
        "while_before_dereference",
        "loop_before_dereference",
        "for_before_dereference",
        "match_before_dereference",
        "parenthesized_if_binary",
        "parenthesized_block_binary",
        "parenthesized_block_cast",
        "initializer_if_binary",
        "return_if_binary",
        "binary_right_block",
        "binary_right_if",
        "argument_if_binary",
        "prefixed_block_value",
        "block_method_continuation",
        "if_method_continuation",
        "if_field_continuation",
        "parenthesized_index",
        "if_arm_before_negative_pattern",
        "block_arm_method_continuation",
        "control_flow_arms_can_omit_commas",
        "explicit_semicolons_discard_block_values",
        "unparenthesized_block_tail_is_a_value",
        "parenthesized_block_tail_is_a_value",
        "unparenthesized_if_tail_is_a_value",
        "diverging_implicit_statement",
        "return_still_exits_before_later_effects",
        "discarded_affine_block_values_drop_once",
    ];
    let directory = directory();
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
fn block_statements_stop_before_ambiguous_prefix_expressions() {
    for statement in [
        "{}",
        "if true {}",
        "if let Some(value)=input {}",
        "while false {}",
        "while let Some(value)=input {}",
        "loop{break;}",
        "for value in 0..1{}",
        "match input{_=>{}}",
    ] {
        let source = format!("fn f(){{{statement} *reference}}");
        let parsed = syntax::parse(&source).unwrap();
        let body = &parsed.blocks[parsed.functions[0].body.unwrap()];
        assert_eq!(body.statements.len(), 1, "{source}");
        assert!(
            matches!(
                body.statements[0],
                Stmt::Expr {
                    semicolon: false,
                    ..
                }
            ),
            "{source}"
        );
        assert!(
            matches!(&parsed.exprs[body.tail.unwrap()].kind, Expr::Unary { op, .. } if op=="*"),
            "{source}"
        );
    }
}

#[test]
fn written_semicolons_are_preserved_without_changing_tail_values() {
    let parsed = syntax::parse("fn f(){ {} {1}; if true {} {42} }").unwrap();
    let body = &parsed.blocks[parsed.functions[0].body.unwrap()];
    assert_eq!(body.statements.len(), 3);
    assert!(matches!(
        body.statements[0],
        Stmt::Expr {
            semicolon: false,
            ..
        }
    ));
    assert!(matches!(
        body.statements[1],
        Stmt::Expr {
            semicolon: true,
            ..
        }
    ));
    assert!(matches!(
        body.statements[2],
        Stmt::Expr {
            semicolon: false,
            ..
        }
    ));
    assert!(matches!(
        parsed.exprs[body.tail.unwrap()].kind,
        Expr::Block(_)
    ));
}

#[test]
fn parenthesized_blocks_keep_call_and_index_expression_syntax() {
    let calls = syntax::parse("fn f(){(if true {callee}else{callee})(42)}").unwrap();
    let body = &calls.blocks[calls.functions[0].body.unwrap()];
    assert!(body.statements.is_empty());
    assert!(matches!(
        calls.exprs[body.tail.unwrap()].kind,
        Expr::Call { .. }
    ));
    let index = syntax::parse("fn f(){({[42]})[0]}").unwrap();
    let body = &index.blocks[index.functions[0].body.unwrap()];
    assert!(body.statements.is_empty());
    assert!(matches!(
        index.exprs[body.tail.unwrap()].kind,
        Expr::Index { .. }
    ));
}

fn rust_rejects(source: &str, reason: &str) {
    let directory = directory();
    std::fs::write(directory.join("case.rs"), source).unwrap();
    let result = Command::new("rustc")
        .args(["--edition=2024", "--crate-type=lib", "--emit=metadata"])
        .arg(directory.join("case.rs"))
        .arg("-o")
        .arg(directory.join("case.rmeta"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    assert!(!result.status.success(), "Rust accepted {source}");
    assert!(
        String::from_utf8_lossy(&result.stderr).contains(reason),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn implicit_block_statements_require_unit_even_in_unused_generic_bodies() {
    for source in [
        "fn bad()->i64{{1i64} -1i64;0}",
        "fn bad()->i64{if true {1i64}else{2i64} 0}",
        "fn bad()->i64{match 0i64{_=>1i64} 0}",
        "fn bad<T>(value:T)->i64{{1i64} 0} pub fn probe()->i64{42}",
        "fn bad<T>(value:T)->i64{return 0;{1i64} 0} pub fn probe()->i64{42}",
        "fn bad()->i64{return 0;{1i64} 0}",
    ] {
        match compiler::check_bundle(&bundle(source)) {
            Ok(_) => panic!("implicit non-unit statement was accepted: {source}"),
            Err(problem) => {
                assert_eq!(problem.code, "E_TYPE", "{problem:?}");
                assert!(
                    problem.message.contains("without `;` must have unit type"),
                    "{problem:?}"
                );
                assert_eq!(problem.path, "block_statements.rs");
            }
        }
        rust_rejects(source, "E0308");
    }
}

#[test]
fn grouping_and_postfix_values_still_require_explicit_statement_terminators() {
    for (source, reason) in [
        ("fn bad(){({}) {}}", "expected `;`"),
        ("fn bad(){if true {}.clone() {}}", "expected `;`"),
        (
            "fn bad()->i64{match 0i64{0=>({42i64}) 1=>1i64,_=>2i64}}",
            "expected `,`",
        ),
        (
            "fn bad()->i64{match 0i64{0=>{42i64}.clone() 1=>1i64,_=>2i64}}",
            "expected `,`",
        ),
        (
            "fn bad()->i64{if true{42i64}else{0i64} as i64}",
            "parenthesize",
        ),
        ("fn bad()->i64{if true{} +42i64}", "expected an expression"),
    ] {
        match syntax::parse(source) {
            Ok(_) => panic!("malformed statement boundary was accepted: {source}"),
            Err(problem) => {
                assert_eq!(problem.code, "E_PARSE", "{problem:?}");
                assert!(problem.message.contains(reason), "{problem:?}");
            }
        }
        rust_rejects(source, "error");
    }
}

#[test]
fn postfix_question_mark_continues_block_expression_syntax() {
    let parsed = syntax::parse("fn pending()->Option<i64>{{Some(1i64)}?;Some(42)}").unwrap();
    let body = &parsed.blocks[parsed.functions[0].body.unwrap()];
    assert_eq!(body.statements.len(), 1);
    let Stmt::Expr { value, semicolon } = body.statements[0] else {
        panic!("missing expression statement");
    };
    assert!(semicolon);
    assert!(matches!(parsed.exprs[value].kind, Expr::Try(_)));
    // This is parser evidence only. `?`, if-let, and while-let execution remain
    // outside the implemented subset; the runtime oracle excludes those forms.
}
