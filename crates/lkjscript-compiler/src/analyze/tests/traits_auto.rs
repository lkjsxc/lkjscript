use super::*;

#[test]
fn structural_auto_traits_cover_nested_products_and_reject_resources_and_cycles() {
    let nested_products = "product/\nname/\nLeaf\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nStr\n/type\n/field\n/fields\n/product\nproduct/\nname/\nNest\n/name\nfields/\nfield/\nname/\nitems\n/name\ntype/\nOption\nResult\nList\nProduct\nLeaf\nI64\n/type\n/field\n/fields\n/product\n";
    let value = "product-value/\nNest\nfield/\nitems\nnone/\nResult\nList\nProduct\nLeaf\nI64\n/none\n/field\n/product-value";
    let copy_source = format!(
        "{nested_products}{}{}",
        bounded_identity("accept", "Copy"),
        main_source("Product\nNest", &format!("accept/\n{value}\n/accept"))
    );
    analyze_one(&copy_source).expect("nested immutable GC handles are Copy within one worker");
    for worker_trait in ["Send", "Sync"] {
        let source = format!(
            "{nested_products}{}{}",
            bounded_identity("accept", worker_trait),
            main_source("Product\nNest", &format!("accept/\n{value}\n/accept"))
        );
        assert!(
            analysis_error(&source).contains("does not satisfy trait"),
            "worker-local GC product unexpectedly satisfied {worker_trait}"
        );
    }
    for worker_trait in ["Send", "Sync"] {
        let source = format!(
            "{}{}",
            bounded_identity("accept-scalar", worker_trait),
            main_source("I64", "accept-scalar/\n7\n/accept-scalar")
        );
        analyze_one(&source).unwrap_or_else(|error| {
            panic!("scalar {worker_trait} fact unexpectedly failed: {error}")
        });
    }
    for ty in ["Buf", "Handle"] {
        let body = if ty == "Buf" {
            "buf-new/\n0\n/buf-new"
        } else {
            "stdin-handle/\n/stdin-handle"
        };
        let source = format!(
            "{}{}",
            bounded_identity("send-value", "Send"),
            main_source(ty, &format!("send-value/\n{body}\n/send-value"))
        );
        assert!(analysis_error(&source).contains("does not satisfy trait Send"));
    }

    let recursive = "product/\nname/\nRecursive\n/name\nfields/\nfield/\nname/\nnext\n/name\ntype/\nOption\nProduct\nRecursive\n/type\n/field\n/fields\n/product\n";
    let source = format!(
        "{recursive}{}{}",
        bounded_identity("copy-recursive", "Copy"),
        main_source("Product\nRecursive", "copy-recursive/\nproduct-value/\nRecursive\nfield/\nnext\nnone/\nProduct\nRecursive\n/none\n/field\n/product-value\n/copy-recursive")
    );
    assert!(analysis_error(&source).contains("recursive product cycle"));

    let mut deep = String::new();
    for index in 0..20 {
        let field_type = if index == 19 {
            "I64".to_string()
        } else {
            format!("Option\nProduct\nP{}", index + 1)
        };
        deep.push_str(&format!(
            "product/\nname/\nP{index}\n/name\nfields/\nfield/\nname/\nnext\n/name\ntype/\n{field_type}\n/type\n/field\n/fields\n/product\n"
        ));
    }
    deep.push_str(&bounded_identity("copy-deep", "Copy"));
    deep.push_str(&main_source(
        "Product\nP0",
        "copy-deep/\nproduct-value/\nP0\nfield/\nnext\nnone/\nProduct\nP1\n/none\n/field\n/product-value\n/copy-deep",
    ));
    let first = analysis_error(&deep);
    let second = analysis_error(&deep);
    assert!(first.contains("trait solver depth exceeded"));
    assert_eq!(first, second);
}
