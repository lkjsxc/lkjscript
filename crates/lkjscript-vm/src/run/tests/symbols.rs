use super::*;

#[test]
fn returned_symbol_retains_text_after_artifact_release() {
    let returned = {
        let mut chunk = Chunk::new();
        let symbol = chunk.add_const(Constant::Symbol("retained-symbol".into()));
        chunk.main.emit_op_u16(Op::LoadConst, symbol.0);
        chunk.main.emit(Op::Return);
        let chunk = validate_chunk(chunk, &ValidationLimits::default()).expect("symbol validates");
        Vm::new(
            &chunk,
            NullJit,
            crate::ExecutionInputs::default(),
            ExecutionConfig::default(),
        )
        .run()
    };
    assert!(matches!(
        returned,
        ExecutionOutcome::Returned(value) if value.as_str() == Some("retained-symbol")
    ));
}
