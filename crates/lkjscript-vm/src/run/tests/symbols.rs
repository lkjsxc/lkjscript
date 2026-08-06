use super::*;

#[test]
fn returned_symbol_retains_text_after_artifact_release() {
    let returned = {
        let mut chunk = Chunk::new();
        let symbol = chunk
            .add_const(Constant::Symbol("retained-symbol".into()))
            .expect("add symbol constant");
        chunk.main.emit_op_u64(Op::LoadConst, symbol.0);
        chunk.main.emit(Op::Return);
        let chunk =
            validate_chunk(chunk, ValidationPolicy::Unrestricted).expect("symbol validates");
        let config = ExecutionConfig {
            max_heap_bytes: 0,
            max_allocations: 0,
            ..ExecutionConfig::default()
        };
        Vm::new(&chunk, NullJit, crate::ExecutionInputs::default(), config).run()
    };
    assert!(matches!(
        returned,
        ExecutionOutcome::Returned(value) if value.as_str() == Some("retained-symbol")
    ));
}
