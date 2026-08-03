use super::*;

#[test]
fn structural_destinations_reject_incomplete_and_double_initialization(
) -> Result<(), Box<dyn std::error::Error>> {
    let field = ty(3, StructuralKind::I64);
    let product = ty(4, StructuralKind::Product);
    let aggregate = StructuralAggregateDescriptor::new(
        9,
        product,
        StructuralAggregateKind::Product,
        vec![field],
    );
    let incomplete = destination_plan(&aggregate, |builder, block, destination, field_owner| {
        sc(
            builder,
            block,
            StructuralOperation::Drop(field),
            vec![field_owner],
        )?;
        let unit = builder.unit(block)?;
        builder.return_value(block, unit)?;
        let _ = destination;
        Ok(())
    })?;
    assert!(matches!(
        verify(incomplete),
        Err(VerificationError::IncompleteStructuralDestination(_))
    ));

    let doubled = destination_plan(&aggregate, |builder, block, destination, field_owner| {
        let initialized = sc(
            builder,
            block,
            StructuralOperation::DestinationInitialize {
                aggregate: aggregate.clone(),
                storage: StructuralStorageRoute::Unique,
                field: 0,
            },
            vec![destination, field_owner],
        )?;
        let raw = builder.i64_const(block, 2)?;
        let second = sc(
            builder,
            block,
            StructuralOperation::PublishI64 {
                value_type: aggregate.fields()[0],
                storage: StructuralStorageRoute::Unique,
            },
            vec![raw],
        )?;
        sc(
            builder,
            block,
            StructuralOperation::DestinationInitialize {
                aggregate: aggregate.clone(),
                storage: StructuralStorageRoute::Unique,
                field: 0,
            },
            vec![initialized, second],
        )?;
        let unit = builder.unit(block)?;
        builder.return_value(block, unit)
    })?;
    assert!(matches!(
        verify(doubled),
        Err(VerificationError::TypeMismatch("structural runtime call"))
    ));
    Ok(())
}
