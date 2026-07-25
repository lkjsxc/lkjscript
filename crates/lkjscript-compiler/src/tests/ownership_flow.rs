use super::*;

#[test]
fn initial_owned_buf_slice_accepts_nll_mutation_move_and_return() {
    let source = ownership_source(
        "let/\nbind/\nb\nowned-buf-new/\n2\n/owned-buf-new\n/bind\ndo/\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\nowned-buf-len/\nr\n/owned-buf-len\n/let\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\ndo/\nowned-buf-set/\nm\n0\n65\n/owned-buf-set\nmove/\nb\n/move\n/do\n/let\n/do\n/let",
        "Owned Buf",
    );
    let program = compile_source(&source, "owned-valid.lkjscript", &Limits::default())
        .expect("valid Owned Buf safe island");
    assert!(program
        .ssa()
        .program()
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .any(|instruction| matches!(instruction.kind, lkjscript_ir::InstructionKind::Move { .. })));

    let shared_pair = ownership_source(
        "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nlet/\nbind/\nr1\nborrow/\nb\n/borrow\n/bind\nbind/\nr2\nborrow/\nb\n/borrow\n/bind\ndo/\nowned-buf-len/\nr1\n/owned-buf-len\nowned-buf-len/\nr2\n/owned-buf-len\n/do\n/let\n/let",
        "I64",
    );
    compile_source(
        &shared_pair,
        "owned-shared-pair.lkjscript",
        &Limits::default(),
    )
    .expect("overlapping shared loans must be accepted");

    let equal_branch = ownership_source(
        "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nif/\ntrue\nmove/\nb\n/move\nmove/\nb\n/move\n/if\n/let",
        "Owned Buf",
    );
    compile_source(
        &equal_branch,
        "owned-equal-branch.lkjscript",
        &Limits::default(),
    )
    .expect("equal branch move states must join");

    let branch_local_result = ownership_source(
        "if/\ntrue\nlet/\nbind/\na\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nmove/\na\n/move\n/let\nlet/\nbind/\nb\nowned-buf-new/\n2\n/owned-buf-new\n/bind\nmove/\nb\n/move\n/let\n/if",
        "Owned Buf",
    );
    compile_source(
        &branch_local_result,
        "owned-branch-local-result.lkjscript",
        &Limits::default(),
    )
    .expect("transferred branch-local owners must canonicalize at the result join");

    let constant_false_loop = ownership_source("while/\nfalse\nunit\n/while", "Unit");
    compile_source(
        &constant_false_loop,
        "constant-false-loop.lkjscript",
        &Limits::default(),
    )
    .expect("branch simplification must clear a stale loop-header marker");
}

#[test]
fn initial_owned_buf_slice_rejects_affine_and_alias_failures() {
    let cases = [
        ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nb\n/let", "Owned Buf", "loaded or copied"),
        ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nmove/\nb\n/move\nmove/\nb\n/move\n/do\n/let", "Owned Buf", "double move"),
        ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\ndo/\nmove/\nb\n/move\nowned-buf-len/\nr\n/owned-buf-len\n/do\n/let\n/let", "I64", "while it is borrowed"),
        ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nif/\ntrue\ndo/\nmove/\nb\n/move\nunit\n/do\nunit\n/if\n/let", "Unit", "branch join"),
        ("borrow/\nowned-buf-new/\n1\n/owned-buf-new\n/borrow", "Unit", "whole Owned Buf local"),
        ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nmove/\nb\n/move\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/do\n/let", "I64", "after move"),
        ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\ndo/\nowned-buf-len/\nr\n/owned-buf-len\nowned-buf-set/\nm\n0\n1\n/owned-buf-set\n/do\n/let\n/let\n/let", "Unit", "conflicting shared and exclusive"),
        ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nwhile/\nfalse\nmove/\nb\n/move\n/while\n/let", "Unit", "loop-carried"),
        ("let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nsome/\nborrow/\nb\n/borrow\n/some\nunit\n/do\n/let", "Unit", "cannot be stored in List or Option"),
    ];
    for (body, result, diagnostic) in cases {
        let source = ownership_source(body, result);
        let error = compile_source(&source, "owned-invalid.lkjscript", &Limits::default())
            .expect_err("invalid ownership source must fail")
            .to_string();
        assert!(error.contains(diagnostic), "{diagnostic}: {error}");
    }
}
