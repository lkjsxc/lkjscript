use super::*;

#[test]
fn removed_numeric_vocabulary_and_non_binary_arithmetic_fail() {
    for ty in [
        "I32", "U32", "U64", "F32", "I128", "U8", "F16", "i32", "i64", "u32", "u64", "f32", "f64",
        "i128", "Int", "Float",
    ] {
        let source = format!("main/\nsig/\n->\n{ty}\n/sig\n1\n/main\n");
        let error = compile_source(&source, "removed-type.lkjscript", &Limits::default())
            .expect_err("removed numeric type must fail")
            .to_string();
        assert!(
            error.contains("unsupported numeric type"),
            "wrong diagnostic for {ty}: {error}"
        );
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
            compile_source(&source, "removed-op.lkjscript", &Limits::default()).is_err(),
            "accepted operator {name}"
        );
    }
    let variadic = unit_main("+/\n1\n2\n3\n/+");
    assert!(compile_source(&variadic, "arity.lkjscript", &Limits::default()).is_err());
}
