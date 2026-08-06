use super::*;

pub(super) fn borrow<J: RuntimeTier>(vm: &mut Vm<'_, J>, exclusive: bool) -> Result<()> {
    let view_representation = StructuralRepresentationId::new(vm.read_u64()?);
    let view_type =
        representation_type(vm.chunk, view_representation, StructuralValueCategory::View)?;
    let owner_value = vm.pop()?;
    let (owner, record) = invocation(vm)?.owner(owner_value)?;
    if record.value_type != view_type
        || !same_representation_type(vm.chunk, record.representation, view_representation)?
    {
        return Err(Error::msg(
            "structural borrow representation does not match its owner",
        ));
    }
    let view = invocation_mut(vm)?
        .runtime
        .borrow_projected(
            owner,
            record.value_type,
            StructuralProjection::Field {
                path: StructuralFieldPath::root(),
                expected: record.value_type,
            },
            exclusive,
        )
        .map_err(map_value_error)?;
    match invocation_mut(vm)?.register_view(view, view_representation, view_type, false) {
        Ok(value) => {
            vm.push(value);
            Ok(())
        }
        Err(error) => {
            let _ = invocation_mut(vm)?.runtime.end_view(view);
            Err(error)
        }
    }
}
