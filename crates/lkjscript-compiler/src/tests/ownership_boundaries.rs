use super::*;

#[test]
fn ownership_generic_laundering_and_reference_results_are_rejected() {
    let generic_id = "def/\nname/\nid\n/name\nfn/\nforall/\nt\n/forall\nsig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\nparams/\nx\nt\n/params\nx\n/fn\n/def\n";
    let reference = format!(
        "{generic_id}{}",
        ownership_source(
            "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nlet/\nbind/\nr\nborrow/\nb\n/borrow\n/bind\nlet/\nbind/\nr2\nid/\nr\n/id\n/bind\ndo/\nmove/\nb\n/move\nbyte-slice-length/\nr2\n/byte-slice-length\n/do\n/let\n/let\n/let",
            "i64",
        )
    );
    let owned = format!(
        "{generic_id}{}",
        ownership_source(
            "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nid/\nmove/\nb\n/move\n/id\n/let",
            "byte-vector",
        )
    );
    let generic_with_owned_parameter = "def/\nname/\nconsume-generic\n/name\nfn/\nforall/\nt\n/forall\nsig/\ninputs/\nbyte-vector\nt\n/inputs\noutput/\nt\n/output\n/sig\nparams/\nb\nbyte-vector\nx\nt\n/params\nx\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nconsume-generic/\nmove/\nb\n/move\n7\n/consume-generic\n/let\n/main\n";
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
    let valid = "def/\nname/\nread-owned\n/name\nfn/\nsig/\ninputs/\nbyte-slice\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nr\nbyte-slice\n/params\nbyte-slice-length/\nr\n/byte-slice-length\n/fn\n/def\ndef/\nname/\nwrite-owned\n/name\nfn/\nsig/\ninputs/\nbyte-slice-mut\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\nr\nbyte-slice-mut\n/params\nbyte-slice-mut-set-byte/\nr\n0\n1\n/byte-slice-mut-set-byte\n/fn\n/def\ndef/\nname/\nfresh-owned\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\nbyte-vector\n/output\n/sig\nparams/\nn\ni64\n/params\nnew-byte-vector/\nn\n/new-byte-vector\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nb\nfresh-owned/\n1\n/fresh-owned\n/bind\nread-owned/\nborrow/\nb\n/borrow\n/read-owned\n/let\n/main\n";
    compile_source(valid, "ownership-signatures.lkjscript", &Limits::default())
        .expect("Ref/RefMut parameters and Owned return must remain valid");

    let consumed_ref_mut_before_safepoint = "def/\nname/\nwrite-then-allocate\n/name\nfn/\nsig/\ninputs/\nbyte-slice-mut\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nr\nbyte-slice-mut\n/params\ndo/\nbyte-slice-mut-set-byte/\nr\n0\n1\n/byte-slice-mut-set-byte\nlet/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/let\n/do\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    compile_source(
        consumed_ref_mut_before_safepoint,
        "consumed-ref-mut-frame.lkjscript",
        &Limits::default(),
    )
    .expect("consumed RefMut must leave later semantic frame states");

    let invalid = "def/\nname/\nreturn-ref\n/name\nfn/\nsig/\ninputs/\nbyte-slice\n/inputs\noutput/\nbyte-slice\n/output\n/sig\nparams/\nr\nbyte-slice\n/params\nr\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    let error = compile_source(invalid, "reference-return.lkjscript", &Limits::default())
        .expect_err("reference return must fail")
        .to_string();
    assert!(error.contains("cannot be returned"), "{error}");
}

#[test]
fn ownership_types_cannot_escape_into_products_or_collections() {
    let product_direct = "product/\nname/\nbad\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nbyte-vector\n/type\n/field\n/fields\n/product\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    let product_nested = "product/\nname/\nbad-nested\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\noption\nbyte-slice\n/type\n/field\n/fields\n/product\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    let list = "main/\nsig/\ninputs/\n/inputs\noutput/\nlist/\nbyte-vector\n/list\n/output\n/sig\nunit\n/main\n";
    let option = "main/\nsig/\ninputs/\n/inputs\noutput/\noption/\nbyte-slice\n/option\n/output\n/sig\nunit\n/main\n";
    let result = "main/\nsig/\ninputs/\n/inputs\noutput/\nresult/\ni64\nbyte-slice-mut\n/result\n/output\n/sig\nunit\n/main\n";
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
