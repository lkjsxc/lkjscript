#[test]
fn structural_plan_identity_propagates_and_tamper_fails() {
    let mut chunk = product_chunk();
    emit_finished_product(&mut chunk);
    emit_product_cleanup(&mut chunk);
    validate_chunk(chunk.clone(), &ValidationLimits::default())
        .expect("exact structural plan validates");

    chunk.main.memory_plan = Some(crate::MemoryPlanId::new([8; 32]));
    assert!(error(chunk).contains("MemoryPlanId does not match"));
}

#[test]
fn owner_view_and_destination_metadata_are_opcode_checked() {
    let mut owner = product_chunk();
    let product = owner.add_const(crate::Constant::I64(0));
    owner.main.emit_op_u16(Op::LoadConst, product.0);
    owner.main.emit_op_u16(Op::StructuralPublish, 1);
    owner.main.emit(Op::Return);
    assert!(error(owner).contains("owner representation"));

    let mut view = product_chunk();
    emit_finished_product(&mut view);
    view.main.emit_op_u8(Op::LoadStructuralOwnerLocal, 1);
    view.main.emit_op_u16(Op::StructuralBorrow, 0);
    view.main.emit(Op::Return);
    assert!(error(view).contains("view representation"));

    let mut destination = product_chunk();
    destination.structural_destinations[0].representation =
        crate::StructuralRepresentationId::new(0);
    destination.main.emit(Op::Unit);
    destination.main.emit(Op::Return);
    assert!(error(destination).contains("destination owner metadata is inconsistent"));
}

#[test]
fn destination_double_init_and_incomplete_finish_fail_closed() {
    let mut double = product_chunk();
    double.main.locals = 1;
    let value = double.add_const(crate::Constant::I64(1));
    double.main.emit_op_u16(Op::StructuralDestinationCreate, 0);
    double.main.emit_op_u8(Op::StoreStructuralLocal, 0);
    for _ in 0..2 {
        double.main.emit_op_u8(Op::TakeStructuralLocal, 0);
        double.main.emit_op_u16(Op::LoadConst, value.0);
        double
            .main
            .emit_op_u16(Op::StructuralDestinationFieldInit, 0);
        double.main.emit_op_u8(Op::StoreStructuralLocal, 0);
    }
    double.main.emit(Op::Unit);
    double.main.emit(Op::Return);
    assert!(error(double).contains("initialized twice"));

    let mut incomplete = product_chunk();
    incomplete
        .main
        .emit_op_u16(Op::StructuralDestinationCreate, 0);
    incomplete
        .main
        .emit_op_u16(Op::StructuralDestinationFinish, 0);
    incomplete.main.emit(Op::Return);
    assert!(error(incomplete).contains("destination finish is incomplete"));
}
