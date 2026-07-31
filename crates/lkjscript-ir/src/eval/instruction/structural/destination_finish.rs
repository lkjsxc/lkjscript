impl Evaluator<'_> {
    fn destination_field_type(
        &self,
        destination: &EvalStructuralDestination,
        field: u16,
    ) -> Result<crate::SsaType, Flow> {
        let memory = &self.program.program().memory;
        let value_type = memory
            .types
            .get(destination.type_id.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == destination.type_id)
            .ok_or_else(|| Flow::Trap("destination type metadata is stale".into()))?;
        let runtime_type = self.structural_type(&value_type.ty)?;
        if runtime_type != destination.value_type {
            return Err(Flow::Trap("destination owner type mismatch".into()));
        }
        let layout = memory
            .layouts
            .get(value_type.layout.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == value_type.layout)
            .ok_or_else(|| Flow::Trap("destination layout metadata is stale".into()))?;
        let index = usize::from(field);
        match &layout.kind {
            crate::StructuralLayoutKind::Product { fields, .. } => fields.get(index),
            crate::StructuralLayoutKind::Enum { variants, .. } => destination
                .active_variant
                .and_then(|active| variants.iter().find(|variant| variant.variant == active))
                .and_then(|variant| variant.fields.get(index)),
            crate::StructuralLayoutKind::String | crate::StructuralLayoutKind::Path => None,
        }
        .cloned()
        .ok_or_else(|| Flow::Trap("destination field is out of range".into()))
    }

    fn finish_explicit_destination(
        &mut self,
        values: &mut [Option<EvalValue>],
        source: ValueId,
        result_ty: &crate::SsaType,
    ) -> Result<EvalValue, Flow> {
        let destination_value = take_value(values, source)?;
        let EvalValue::StructuralDestination(destination) = destination_value else {
            return Err(Flow::Trap("destination finish expects destination".into()));
        };
        let expected = self.structural_type(result_ty)?;
        if destination.value_type != expected {
            restore_slot(
                values,
                source,
                EvalValue::StructuralDestination(destination),
            )?;
            return Err(Flow::Trap("destination finish type mismatch".into()));
        }
        match self.structural.runtime.finish_destination(destination.key) {
            Ok(key) => Ok(EvalValue::StructuralOwner(EvalStructuralOwner {
                key,
                value_type: expected,
            })),
            Err(error) => {
                restore_slot(
                    values,
                    source,
                    EvalValue::StructuralDestination(destination),
                )?;
                Err(map_structural_error(error))
            }
        }
    }
}
