use super::*;

#[test]
fn structural_auto_traits_cover_nested_products_and_reject_resources_and_cycles() {
    let nested_products = "product/\nname/\nleaf\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nstring\n/type\n/field\n/fields\n/product\nproduct/\nname/\nnest\n/name\nfields/\nfield/\nname/\nitems\n/name\ntype/\noption\nresult\nlist\nproduct\nleaf\ni64\n/type\n/field\n/fields\n/product\n";
    let value = "product-value/\nnest\nfield/\nitems\nnone/\nresult\nlist\nproduct\nleaf\ni64\n/none\n/field\n/product-value";
    let copy_source = format!(
        "{nested_products}{}{}",
        bounded_identity("accept", "copy"),
        main_source(
            "product/\nnest\n/product",
            &format!("accept/\n{value}\n/accept")
        )
    );
    analyze_one(&copy_source).expect("nested immutable GC handles are copy within one worker");
    for worker_trait in ["send", "sync"] {
        let source = format!(
            "{nested_products}{}{}",
            bounded_identity("accept", worker_trait),
            main_source(
                "product/\nnest\n/product",
                &format!("accept/\n{value}\n/accept")
            )
        );
        assert!(
            analysis_error(&source).contains("does not satisfy trait"),
            "worker-local GC product unexpectedly satisfied {worker_trait}"
        );
    }
    for worker_trait in ["send", "sync"] {
        let source = format!(
            "{}{}",
            bounded_identity("accept-scalar", worker_trait),
            main_source("i64", "accept-scalar/\n7\n/accept-scalar")
        );
        analyze_one(&source).unwrap_or_else(|error| {
            panic!("scalar {worker_trait} fact unexpectedly failed: {error}")
        });
    }
    let buffer = format!(
        "{}{}",
        bounded_identity("send-value", "send"),
        main_source("buf", "send-value/\nbuf-new/\n0\n/buf-new\n/send-value")
    );
    assert!(analysis_error(&buffer).contains("does not satisfy trait send"));
    let handle = format!(
        "{}{}",
        bounded_identity("send-value", "send"),
        concat!(
            "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\noutput/\nunit\n/output\n/sig\n",
            "params/\nstdio\ncapability/\nstdio\n/capability\n/params\n",
            "do/\nsend-value/\nstandard-input/\nstdio\n/standard-input\n/send-value\nunit\n/do\n/main\n"
        )
    );
    let error = analysis_error(&handle);
    assert!(
        error.contains("ownership/reference generic instantiation is unavailable"),
        "{error}"
    );

    let recursive = "product/\nname/\nrecursive\n/name\nfields/\nfield/\nname/\nnext\n/name\ntype/\noption\nproduct\nrecursive\n/type\n/field\n/fields\n/product\n";
    let source = format!(
        "{recursive}{}{}",
        bounded_identity("copy-recursive", "copy"),
        main_source("product/\nrecursive\n/product", "copy-recursive/\nproduct-value/\nrecursive\nfield/\nnext\nnone/\nproduct\nrecursive\n/none\n/field\n/product-value\n/copy-recursive")
    );
    assert!(analysis_error(&source).contains("recursive product cycle"));

    let mut deep = String::new();
    for index in 0..20 {
        let field_type = if index == 19 {
            "i64".to_string()
        } else {
            format!("option/\nproduct/\np{}\n/product\n/option", index + 1)
        };
        deep.push_str(&format!(
            "product/\nname/\np{index}\n/name\nfields/\nfield/\nname/\nnext\n/name\ntype/\n{field_type}\n/type\n/field\n/fields\n/product\n"
        ));
    }
    deep.push_str(&bounded_identity("copy-deep", "copy"));
    deep.push_str(&main_source(
        "product/\np0\n/product",
        "copy-deep/\nproduct-value/\np0\nfield/\nnext\nnone/\nproduct\np1\n/none\n/field\n/product-value\n/copy-deep",
    ));
    let first = analysis_error(&deep);
    let second = analysis_error(&deep);
    assert!(first.contains("trait solver depth exceeded"));
    assert_eq!(first, second);
}
