use super::*;

#[test]
fn removed_numeric_vocabulary_and_non_binary_arithmetic_fail() {
    for ty in [
        "I64", "F64", "I32", "U32", "U64", "F32", "I128", "U8", "F16", "Int", "Float",
    ] {
        let source =
            format!("main/\nsig/\ninputs/\n/inputs\noutput/\n{ty}\n/output\n/sig\n1\n/main\n");
        let error = compile_source(&source, "removed-type.lkjscript")
            .expect_err("uppercase numeric type must fail")
            .to_string();
        assert!(error.contains("lowercase ASCII kebab-case") || error.contains("is removed"));
    }
    for ty in ["i32", "u32", "u64", "f32", "i128"] {
        let source =
            format!("main/\nsig/\ninputs/\n/inputs\noutput/\n{ty}\n/output\n/sig\n1\n/main\n");
        let error = compile_source(&source, "unsupported-type.lkjscript")
            .expect_err("unsupported numeric width must fail")
            .to_string();
        assert!(error.contains("unsupported numeric type"), "{ty}: {error}");
    }
    for name in [
        "eq",
        "ne",
        "f+",
        "f-",
        "f*",
        "f=",
        "f!=",
        "f<",
        "f<=",
        "f>",
        "f>=",
        "le",
        "ge",
        "=",
        "!=",
        "<",
        "<=",
        ">",
        ">=",
        "i64-from-u32",
        "u32-from-i64",
        "i64-from-i32",
        "i32-from-i64",
    ] {
        let source = unit_main(&format!("{name}/\n1\n2\n/{name}"));
        assert!(
            compile_source(&source, "removed-op.lkjscript").is_err(),
            "accepted operator {name}"
        );
    }
    let variadic = unit_main("add/\n1\n2\n3\n/add");
    assert!(compile_source(&variadic, "arity.lkjscript").is_err());
}
