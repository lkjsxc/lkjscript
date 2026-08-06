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
