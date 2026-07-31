fn validate_region_product_arguments(
    callee: &FunctionProto,
    arguments: &[Kind],
    caller: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    for (index, actual) in arguments.iter().copied().enumerate() {
        let expected = callee
            .parameter_region_products
            .get(index)
            .copied()
            .flatten();
        let valid = match (expected, actual) {
            (Some(expected), Kind::RegionProduct(actual)) => expected == actual,
            (Some(_), _) | (None, Kind::RegionProduct(_)) => false,
            (None, _) => true,
        };
        if !valid {
            return Err(instruction_error(
                caller,
                instruction.op(),
                instruction.offset(),
                "region-product call argument does not match exact metadata",
            ));
        }
    }
    Ok(())
}

fn validate_region_product_return(
    proto: &FunctionProto,
    actual: Kind,
    instruction: DecodedInstruction,
) -> Result<()> {
    let valid = match (proto.return_region_product, actual) {
        (Some(expected), Kind::RegionProduct(actual)) => expected == actual,
        (Some(_), Kind::Any) => true,
        (Some(_), _) | (None, Kind::RegionProduct(_)) => false,
        (None, _) => true,
    };
    if valid {
        Ok(())
    } else {
        Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "region-product return does not match exact metadata",
        ))
    }
}
