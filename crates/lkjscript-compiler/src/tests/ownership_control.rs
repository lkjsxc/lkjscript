use super::*;

#[test]
fn lexical_owned_places_join_without_branch_local_pollution() {
    let valid_local = ownership_source(
        "if/\ntrue\nlet/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/let\nlet/\nbind/\nb\nowned-buf-new/\n2\n/owned-buf-new\n/bind\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/let\n/if",
        "I64",
    );
    compile_source(
        &valid_local,
        "branch-local-owned.lkjscript",
        &Limits::default(),
    )
    .expect("branch-local Owned places must end before the join");

    let valid_reinit = ownership_source(
        "var/\nname/\nb\n/name\ntype/\nOwned\nBuf\n/type\nowned-buf-new/\n1\n/owned-buf-new\ndo/\nif/\ntrue\nmove/\nb\n/move\nmove/\nb\n/move\n/if\nset/\nb\nowned-buf-new/\n3\n/owned-buf-new\n/set\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/do\n/var",
        "I64",
    );
    compile_source(
        &valid_reinit,
        "branch-reinit-owned.lkjscript",
        &Limits::default(),
    )
    .expect("equal branch moves may be reinitialized after the join");

    let invalid_after_move = ownership_source(
        "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nmove/\nb\n/move\nif/\ntrue\nlet/\nbind/\nlocal\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nunit\n/let\nunit\n/if\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/do\n/let",
        "I64",
    );
    let error = compile_source(
        &invalid_after_move,
        "branch-after-move.lkjscript",
        &Limits::default(),
    )
    .expect_err("branch-local place must not resurrect a moved outer place")
    .to_string();
    assert!(
        error.contains("after move"),
        "wrong move diagnostic: {error}"
    );

    let invalid_reinit = ownership_source(
        "var/\nname/\nb\n/name\ntype/\nOwned\nBuf\n/type\nowned-buf-new/\n1\n/owned-buf-new\nset/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/set\n/var",
        "Unit",
    );
    let error = compile_source(
        &invalid_reinit,
        "owned-reinit-before-move.lkjscript",
        &Limits::default(),
    )
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
        "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\nmove/\nb\n/move\n/do\n/let",
        "Owned Buf",
    );
    compile_source(&direct, "temporary-borrow.lkjscript", &Limits::default())
        .expect("direct temporary borrow must end after the operation");

    let unsupported = [
        "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nborrow/\nb\n/borrow\nunit\n/do\n/let",
        "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nowned-buf-len/\nif/\ntrue\nborrow/\nb\n/borrow\nborrow/\nb\n/borrow\n/if\n/owned-buf-len\n/let",
        "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nowned-buf-len/\ndo/\nborrow/\nb\n/borrow\n/do\n/owned-buf-len\n/let",
    ];
    for body in unsupported {
        let source = ownership_source(body, if body.contains("unit") { "Unit" } else { "I64" });
        let error = compile_source(
            &source,
            "unsupported-borrow-placement.lkjscript",
            &Limits::default(),
        )
        .expect_err("unsupported Borrow placement must fail")
        .to_string();
        assert!(
            error.contains("exact direct reference argument or direct let initializer"),
            "wrong Borrow placement diagnostic: {error}"
        );
    }

    let borrow_then_move_call = "def/\nname/\nobserve-and-take\n/name\nfn/\nsig/\nRef\nBuf\nOwned\nBuf\n->\nI64\n/sig\nparams/\nr\nRef/\nBuf\n/Ref\nb\nOwned/\nBuf\n/Owned\n/params\nowned-buf-len/\nr\n/owned-buf-len\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nobserve-and-take/\nborrow/\nb\n/borrow\nmove/\nb\n/move\n/observe-and-take\n/let\n/main\n";
    let error = compile_source(
        borrow_then_move_call,
        "temporary-full-call.lkjscript",
        &Limits::default(),
    )
    .expect_err("temporary loan must cover all call arguments")
    .to_string();
    assert!(error.contains("while it is borrowed"), "{error}");
}
