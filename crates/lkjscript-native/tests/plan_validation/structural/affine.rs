use super::*;

#[test]
fn structural_affine_plans_reject_wrong_type_copy_missing_drop_and_live_loan(
) -> Result<(), Box<dyn std::error::Error>> {
    let wrong = owner_plan(|builder, block, owner, _| {
        let other = ty(2, StructuralKind::String);
        let moved = sc(
            builder,
            block,
            StructuralOperation::Move(other),
            vec![owner],
        )?;
        builder.return_value(block, moved)
    })?;
    assert!(matches!(
        verify(wrong),
        Err(VerificationError::TypeMismatch("structural runtime call"))
    ));

    let copied = owner_plan(|builder, block, owner, value_type| {
        let moved = sc(
            builder,
            block,
            StructuralOperation::Move(value_type),
            vec![owner],
        )?;
        sc(
            builder,
            block,
            StructuralOperation::Drop(value_type),
            vec![owner],
        )?;
        builder.return_value(block, moved)
    })?;
    assert!(matches!(
        verify(copied),
        Err(VerificationError::ValueNotAvailable(_))
    ));

    let missing = owner_plan(|builder, block, _, _| {
        let unit = builder.unit(block)?;
        builder.return_value(block, unit)
    })?;
    assert!(matches!(
        verify(missing),
        Err(VerificationError::LiveAffineValue(_))
    ));

    let borrowed = owner_plan(|builder, block, owner, value_type| {
        let view_type = StructuralViewType::new(7, value_type, value_type, false);
        let projection = StructuralProjectionDescriptor::new(
            view_type,
            StructuralProjectionKind::Field,
            Vec::new(),
        );
        sc(
            builder,
            block,
            StructuralOperation::Borrow { projection },
            vec![owner],
        )?;
        sc(
            builder,
            block,
            StructuralOperation::Drop(value_type),
            vec![owner],
        )?;
        let unit = builder.unit(block)?;
        builder.return_value(block, unit)
    })?;
    assert!(matches!(
        verify(borrowed),
        Err(VerificationError::LiveLoan(_))
    ));
    Ok(())
}

#[test]
fn observed_structural_locals_support_only_nonconsuming_calls(
) -> Result<(), Box<dyn std::error::Error>> {
    let valid = owner_plan(|builder, block, owner, value_type| {
        let local = builder.create_local(ValueType::StructuralOwner(value_type))?;
        builder.write_local(block, local, owner)?;
        let observed = builder.observe_local(block, local)?;
        sc(
            builder,
            block,
            StructuralOperation::PayloadLength(value_type),
            vec![observed],
        )?;
        let owned = builder.read_local(block, local)?;
        sc(
            builder,
            block,
            StructuralOperation::Drop(value_type),
            vec![owned],
        )?;
        let unit = builder.unit(block)?;
        builder.return_value(block, unit)
    })?;
    verify(valid)?;

    let invalid = owner_plan(|builder, block, owner, value_type| {
        let local = builder.create_local(ValueType::StructuralOwner(value_type))?;
        builder.write_local(block, local, owner)?;
        let observed = builder.observe_local(block, local)?;
        sc(
            builder,
            block,
            StructuralOperation::Drop(value_type),
            vec![observed],
        )?;
        let unit = builder.unit(block)?;
        builder.return_value(block, unit)
    })?;
    assert!(matches!(
        verify(invalid),
        Err(VerificationError::TypeMismatch(
            "observed structural local use"
        ))
    ));
    Ok(())
}
