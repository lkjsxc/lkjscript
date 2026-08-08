use super::*;

#[test]
fn lexical_owned_places_join_without_branch_local_pollution() {
    let valid_local = ownership_source(
        "if/\ntrue\nlet/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/let\nlet/\nbind/\nb\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/let\n/if",
        "i64",
    );
    compile_source(&valid_local, "branch-local-owned.lkjscript")
        .expect("branch-local Owned places must end before the join");

    let valid_reinit = ownership_source(
        "var/\nname/\nb\n/name\ntype/\nbyte-vector\n/type\nnew-byte-vector/\n1\n/new-byte-vector\ndo/\nif/\ntrue\nmove/\nb\n/move\nmove/\nb\n/move\n/if\nset/\nb\nnew-byte-vector/\n3\n/new-byte-vector\n/set\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/do\n/var",
        "i64",
    );
    compile_source(&valid_reinit, "branch-reinit-owned.lkjscript")
        .expect("equal branch moves may be reinitialized after the join");

    let invalid_after_move = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\nmove/\nb\n/move\nif/\ntrue\nlet/\nbind/\nlocal\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nunit\n/let\nunit\n/if\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/do\n/let",
        "i64",
    );
    let error = compile_source(&invalid_after_move, "branch-after-move.lkjscript")
        .expect_err("branch-local place must not resurrect a moved outer place")
        .to_string();
    assert!(
        error.contains("after move"),
        "wrong move diagnostic: {error}"
    );

    let invalid_reinit = ownership_source(
        "var/\nname/\nb\n/name\ntype/\nbyte-vector\n/type\nnew-byte-vector/\n1\n/new-byte-vector\nset/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/set\n/var",
        "unit",
    );
    let error = compile_source(&invalid_reinit, "owned-reinit-before-move.lkjscript")
        .expect_err("initialized Owned var cannot be overwritten")
        .to_string();
    assert!(
        error.contains("only reinitialization after move"),
        "{error}"
    );
}

#[test]
fn temporary_borrows_have_only_direct_supported_placements() {
    let direct = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\nmove/\nb\n/move\n/do\n/let",
        "byte-vector",
    );
    compile_source(&direct, "temporary-borrow.lkjscript")
        .expect("direct temporary borrow must end after the operation");

    let unsupported = [
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\nborrow/\nb\n/borrow\nunit\n/do\n/let",
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nbyte-slice-length/\nif/\ntrue\nborrow/\nb\n/borrow\nborrow/\nb\n/borrow\n/if\n/byte-slice-length\n/let",
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nbyte-slice-length/\ndo/\nborrow/\nb\n/borrow\n/do\n/byte-slice-length\n/let",
    ];
    for body in unsupported {
        let source = ownership_source(body, if body.contains("unit") { "unit" } else { "i64" });
        let error = compile_source(&source, "unsupported-borrow-placement.lkjscript")
            .expect_err("unsupported Borrow placement must fail")
            .to_string();
        assert!(
            error.contains("exact direct reference argument or direct let initializer"),
            "wrong Borrow placement diagnostic: {error}"
        );
    }

    let borrow_then_move_call = "def/\nname/\nobserve-and-take\n/name\nfn/\nsig/\ninputs/\nbyte-slice\nbyte-vector\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nr\nbyte-slice\nb\nbyte-vector\n/params\nbyte-slice-length/\nr\n/byte-slice-length\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nobserve-and-take/\nborrow/\nb\n/borrow\nmove/\nb\n/move\n/observe-and-take\n/let\n/main\n";
    let error = compile_source(borrow_then_move_call, "temporary-full-call.lkjscript")
        .expect_err("temporary loan must cover all call arguments")
        .to_string();
    assert!(error.contains("while it is borrowed"), "{error}");
}

#[test]
fn lexical_reference_liveness_covers_calls_scopes_and_branch_paths() {
    let lexical_then_move_call = "def/\nname/\nobserve-and-take\n/name\nfn/\nsig/\ninputs/\nbyte-slice\nbyte-vector\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nr\nbyte-slice\nb\nbyte-vector\n/params\nbyte-slice-length/\nr\n/byte-slice-length\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\nobserve-and-take/\nr\nmove/\nb\n/move\n/observe-and-take\n/let\n/let\n/main\n";
    let error = compile_source(lexical_then_move_call, "lexical-full-call.lkjscript")
        .expect_err("an evaluated lexical reference must stay borrowed through its call")
        .to_string();
    assert!(error.contains("while it is borrowed"), "{error}");

    let outer_after_mutable = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\ndo/\nvar/\nname/\nx\n/name\ntype/\nbyte-vector\n/type\nmove/\nb\n/move\nunit\n/var\nbyte-slice-length/\nr\n/byte-slice-length\n/do\n/let\n/let",
        "i64",
    );
    let error = compile_source(&outer_after_mutable, "outer-reference-after-var.lkjscript")
        .expect_err("a mutable-local initializer must retain the enclosing continuation")
        .to_string();
    assert!(error.contains("while it is borrowed"), "{error}");

    let interleaved_last_uses = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\nbind/\nr1\nborrow/\nb\n/borrow\n/bind\nbind/\nr2\nborrow/\nb\n/borrow\n/bind\ndo/\nbyte-slice-length/\nr1\n/byte-slice-length\nbyte-slice-length/\nr2\n/byte-slice-length\nmove/\nb\n/move\n/do\n/let\n/let",
        "byte-vector",
    );
    compile_source(
        &interleaved_last_uses,
        "interleaved-reference-uses.lkjscript",
    )
    .expect("each shared loan must expire after its last reachable use");

    let divergent_move = "def/\nname/\ntake-or-observe\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nbyte-vector\n/output\n/sig\nparams/\n/params\nlet/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\ndo/\nif/\ntrue\nreturn/\nmove/\nb\n/move\n/return\nunit\n/if\nbyte-slice-length/\nr\n/byte-slice-length\nmove/\nb\n/move\n/do\n/let\n/let\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    analyze_source(divergent_move)
        .expect("a terminating branch must not inherit the post-conditional reference use");

    let live_after_join = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\ndo/\nif/\ntrue\ndo/\nmove/\nb\n/move\nunit\n/do\nunit\n/if\nbyte-slice-length/\nr\n/byte-slice-length\n/do\n/let\n/let",
        "i64",
    );
    let error = analyze_source(&live_after_join)
        .expect_err("a reachable branch cannot move an owner needed after the join")
        .to_string();
    assert!(error.contains("while it is borrowed"), "{error}");

    let branch_last_use = ownership_source(
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\ndo/\nif/\ntrue\nbyte-slice-length/\nr\n/byte-slice-length\n0\n/if\nmove/\nb\n/move\n/do\n/let\n/let",
        "byte-vector",
    );
    analyze_source(&branch_last_use)
        .expect("branch-local last uses must expire before a post-join owner move");
}

#[test]
fn many_live_references_do_not_require_per_expression_suffix_reconstruction() {
    const REFERENCES: usize = 128;
    const SCALAR_EXPRESSIONS: usize = 512;
    let mut source = String::from(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nbyte-vector\n/output\n/sig\nlet/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\n",
    );
    for index in 0..REFERENCES {
        source.push_str(&format!("bind/\nr-{index}\nborrow/\nb\n/borrow\n/bind\n"));
    }
    source.push_str("do/\n");
    for _ in 0..SCALAR_EXPRESSIONS {
        source.push_str("0\n");
    }
    for index in 0..REFERENCES {
        source.push_str(&format!(
            "byte-slice-length/\nr-{index}\n/byte-slice-length\n"
        ));
    }
    source.push_str("move/\nb\n/move\n/do\n/let\n/let\n/main\n");
    compile_source(&source, "many-live-references.lkjscript")
        .expect("many live shared references must expire before the final owner move");
}
