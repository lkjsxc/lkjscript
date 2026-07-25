use super::*;

#[test]
fn ownership_generic_laundering_and_reference_results_are_rejected() {
    let generic_id = "def/\nname/\nid\n/name\nfn/\nforall/\nT\n/forall\nsig/\nT\n->\nT\n/sig\nparams/\nx\nT\n/params\nx\n/fn\n/def\n";
    let reference = format!(
        "{generic_id}{}",
        ownership_source(
            "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\nlet/\nbind/\nr2\nid/\nr\n/id\n/bind\ndo/\nmove/\nb\n/move\nowned-buf-len/\nr2\n/owned-buf-len\n/do\n/let\n/let\n/let",
            "I64",
        )
    );
    let owned = format!(
        "{generic_id}{}",
        ownership_source(
            "let/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nid/\nmove/\nb\n/move\n/id\n/let",
            "Owned Buf",
        )
    );
    let generic_with_owned_parameter = "def/\nname/\nconsume-generic\n/name\nfn/\nforall/\nT\n/forall\nsig/\nOwned\nBuf\nT\n->\nT\n/sig\nparams/\nb\nOwned/\nBuf\n/Owned\nx\nT\n/params\nx\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nconsume-generic/\nmove/\nb\n/move\n7\n/consume-generic\n/let\n/main\n";
    for source in [reference, owned, generic_with_owned_parameter.into()] {
        let error = compile_source(&source, "generic-owned.lkjscript", &Limits::default())
            .expect_err("generic ownership laundering must fail")
            .to_string();
        assert!(
            error.contains("ownership/reference generic instantiation is unavailable"),
            "wrong generic ownership diagnostic: {error}"
        );
    }
}

#[test]
fn ownership_function_signature_escape_boundary_is_exact() {
    let valid = "def/\nname/\nread-owned\n/name\nfn/\nsig/\nRef\nBuf\n->\nI64\n/sig\nparams/\nr\nRef/\nBuf\n/Ref\n/params\nowned-buf-len/\nr\n/owned-buf-len\n/fn\n/def\ndef/\nname/\nwrite-owned\n/name\nfn/\nsig/\nRefMut\nBuf\n->\nUnit\n/sig\nparams/\nr\nRefMut/\nBuf\n/RefMut\n/params\nowned-buf-set/\nr\n0\n1\n/owned-buf-set\n/fn\n/def\ndef/\nname/\nfresh-owned\n/name\nfn/\nsig/\nI64\n->\nOwned\nBuf\n/sig\nparams/\nn\nI64\n/params\nowned-buf-new/\nn\n/owned-buf-new\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nb\nfresh-owned/\n1\n/fresh-owned\n/bind\nread-owned/\nborrow/\nb\n/borrow\n/read-owned\n/let\n/main\n";
    compile_source(valid, "ownership-signatures.lkjscript", &Limits::default())
        .expect("Ref/RefMut parameters and Owned return must remain valid");

    let consumed_ref_mut_before_safepoint = "def/\nname/\nwrite-then-allocate\n/name\nfn/\nsig/\nRefMut\nBuf\n->\nI64\n/sig\nparams/\nr\nRefMut/\nBuf\n/RefMut\n/params\ndo/\nowned-buf-set/\nr\n0\n1\n/owned-buf-set\nlet/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/let\n/do\n/fn\n/def\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
    compile_source(
        consumed_ref_mut_before_safepoint,
        "consumed-ref-mut-frame.lkjscript",
        &Limits::default(),
    )
    .expect("consumed RefMut must leave later semantic frame states");

    let invalid = "def/\nname/\nreturn-ref\n/name\nfn/\nsig/\nRef\nBuf\n->\nRef\nBuf\n/sig\nparams/\nr\nRef/\nBuf\n/Ref\n/params\nr\n/fn\n/def\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
    let error = compile_source(invalid, "reference-return.lkjscript", &Limits::default())
        .expect_err("reference return must fail")
        .to_string();
    assert!(error.contains("cannot be returned"), "{error}");
}

#[test]
fn ownership_types_cannot_escape_into_products_or_collections() {
    let product_direct = "product/\nname/\nBad\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nOwned\nBuf\n/type\n/field\n/fields\n/product\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
    let product_nested = "product/\nname/\nBadNested\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nOption\nRef\nBuf\n/type\n/field\n/fields\n/product\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
    let list = "main/\nsig/\n->\nList\nOwned\nBuf\n/sig\nunit\n/main\n";
    let option = "main/\nsig/\n->\nOption\nRef\nBuf\n/sig\nunit\n/main\n";
    let result = "main/\nsig/\n->\nResult\nI64\nRefMut\nBuf\n/sig\nunit\n/main\n";
    for source in [product_direct, product_nested, list, option, result] {
        let error = compile_source(source, "stored-owned.lkjscript", &Limits::default())
            .expect_err("ownership storage must fail")
            .to_string();
        assert!(
            error.contains("ownership/reference"),
            "wrong ownership storage diagnostic: {error}"
        );
    }
}
