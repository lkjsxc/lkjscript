//! Scalar Clone stays an ordinary read; nominal Clone and cleanup stay observable.
use crate::{check, embedded, frontend, interpreter, ir, ir_text, source};

fn compile(text: &str) -> Result<ir::CheckedProgram, crate::diagnostic::Diagnostic> {
    let parsed = source::load("clone.rs", Some("$std/lib.rs"), &mut |path: &str| -> Result<Option<Vec<u8>>, String> {
        Ok(if path == "clone.rs" {
            Some(text.as_bytes().to_vec())
        } else {
            embedded::library(path)
        })
    }).unwrap();
    frontend::check_source(&parsed)
}
fn checked(text: &str) -> ir::CheckedProgram { compile(text).unwrap() }
fn entry(program: &ir::Program) -> usize {
    program.functions.iter().position(|function| function.name == "probe").unwrap()
}
fn no_scalar_clone_helpers(program: &ir::Program) {
    for function in &program.functions {
        if function.name.as_str().starts_with("$builtin:clone:") {
            assert!(!matches!(
                program.types[function.result].kind,
                ir::TypeKind::Unit | ir::TypeKind::Bool | ir::TypeKind::Char
                    | ir::TypeKind::Int { .. } | ir::TypeKind::F64
                    | ir::TypeKind::Ref { mutable: false, .. }
            ), "scalar Clone helper survived elaboration: {}", function.name);
        }
    }
}
fn observed(text: &str) {
    let original = checked(text);
    no_scalar_clone_helpers(original.program());
    let program = ir_text::read(&ir_text::write(original.program())).unwrap();
    assert!(program == *original.program());
    let admitted = check::check(program).unwrap();
    let result = interpreter::run(
        admitted.program(), entry(admitted.program()), &[0u64; 0], &mut Vec::<u8>::new(),
    ).unwrap();
    assert!(matches!(
        result.value, interpreter::Scalar::Int { value: 42, bits: 64, signed: true }
    ));
}
fn builtin_scalar_functions_are_reads_without_calls() {
    let names: Vec<String> = vec![
        "()".to_string(), "bool".to_string(), "char".to_string(),
        "u8".to_string(), "u16".to_string(), "u32".to_string(), "u64".to_string(),
        "usize".to_string(), "i8".to_string(), "i16".to_string(), "i32".to_string(),
        "i64".to_string(), "isize".to_string(), "f64".to_string(),
    ];
    for name in names {
        let program = checked(&format!(
            "pub fn probe(value: {name}) -> {name} {{ value.clone() }}"
        ));
        no_scalar_clone_helpers(program.program());
        let function = &program.program().functions[entry(program.program())];
        let mut read = false;
        for instruction in &function.body {
            assert!(!matches!(instruction.op, ir::Op::Call { .. }));
            if let ir::Op::Read { dst, moving: false, .. } = &instruction.op {
                read |= function.locals[*dst].ty == function.result;
            }
        }
        assert!(read, "{name} Clone did not produce an ordinary read");
    }
}
fn method_validation_precedes_the_read_shortcut() {
    let cases: Vec<String> = vec![
        "pub fn probe() -> u8 { 7u8.clone(1u8) }".to_string(),
        "pub fn probe() -> u8 { 7u8.clone::<u8>() }".to_string(),
    ];
    for text in cases {
        let problem = compile(text.as_str()).err().unwrap();
        assert_eq!(problem.code, "E_TYPE");
    }
}
pub fn run(world: &mut World, trace: bool) -> usize {
    if trace { world.stdout().write(b"builtin_scalar_functions_are_reads_without_calls\n"); }
    builtin_scalar_functions_are_reads_without_calls();
    if trace { world.stdout().write(b"scalar_clone_preserves_all_integer_widths_and_float_bits\n"); }
    observed(include_str!("../../tests/conformance/pass/scalar_clone_bits.rs"));
    if trace { world.stdout().write(b"derived_and_generic_clones_preserve_thin_and_fat_views\n"); }
    observed(include_str!("../../tests/conformance/pass/scalar_clone_views.rs"));
    if trace { world.stdout().write(b"copy_nominal_clone_and_clone_from_remain_observable\n"); }
    observed(include_str!("../../tests/conformance/pass/scalar_clone_custom_copy.rs"));
    if trace { world.stdout().write(b"scalar_clone_keeps_temporary_owner_cleanup_and_evaluation_order\n"); }
    observed(include_str!("../../tests/conformance/pass/scalar_clone_temporary_drop.rs"));
    if trace { world.stdout().write(b"method_validation_precedes_the_read_shortcut\n"); }
    method_validation_precedes_the_read_shortcut();
    6usize
}
