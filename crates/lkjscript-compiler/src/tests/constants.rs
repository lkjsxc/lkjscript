use super::*;

#[test]
fn bytecode_constants_preserve_numeric_source_types() {
    let integer = "main/\nsig/\n->\nI64\n/sig\n9223372036854775807\n/main\n";
    let integer = compile_source(integer, "integer.lkjscript", &Limits::default())
        .expect("compile I64 source");
    assert!(integer
        .bytecode()
        .constants()
        .iter()
        .any(|constant| matches!(constant, Constant::I64(i64::MAX))));

    let float = "main/\nsig/\n->\nF64\n/sig\n+/\n2.0\nf64-from-i64-rounded/\n1\n/f64-from-i64-rounded\n/+\n/main\n";
    let float =
        compile_source(float, "float.lkjscript", &Limits::default()).expect("compile F64 source");
    assert!(float
        .bytecode()
        .constants()
        .iter()
        .any(|constant| matches!(constant, Constant::F64(value) if *value == 2.0)));
}
