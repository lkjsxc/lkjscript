//! Resolved identities and lexical shadows remain local to a compilation.
use lkjscript::ir;
#[path = "support/native.rs"]
mod native;

use lkjscript::source::{SourceBundle, SourceInput};

fn compile(source: &str) -> ir::CheckedProgram {
    native::check(&SourceBundle {
        root: "lookup.rs".to_string(),
        library: None,
        files: vec![SourceInput {
            path: "lookup.rs".to_string(),
            text: source.to_string(),
        }],
    })
}

fn run(checked: &ir::CheckedProgram, expected: i64) {
    native::assert_values(checked, &[("probe", expected)]);
}

#[test]
fn declaration_identity_does_not_leak_across_source_bundles() {
    // Identical names have different declaration/function indices in each call.
    let first = compile("fn value()->i64 { 7 } pub fn probe()->i64 { value() }");
    let second = compile(
        "pub fn probe()->i64 { value() } fn unused()->bool { true } fn value()->i64 { 42 }",
    );
    let third =
        compile("fn value()->bool { true } pub fn probe()->i64 { if value() { 9 } else { 0 } }");
    run(&first, 7);
    run(&second, 42);
    run(&third, 9);
}

#[test]
fn lexical_shadows_restore_after_blocks_branches_and_ranges() {
    let bytes = compile(
        "pub fn probe()->i64 {
            let value:i64 = 10;
            let mut sum:i64 = value;
            { let value = value + 1; let value = value + 1; sum = sum + value; }
            if true { let value:i64 = 20; sum = sum + value; }
            else { let value:i64 = 99; sum = sum + value; }
            for value in 0i64..2i64 { sum = sum + value; }
            sum + value
        }",
    );
    run(&bytes, 53);
}
