use super::*;

impl Evaluator<'_> {
    pub(super) fn enum_instruction(
        &mut self,
        function: &crate::Function,
        instruction: &Instruction,
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        match &instruction.kind {
            InstructionKind::EnumValue {
                enum_id,
                variant,
                layout,
                fields,
            } => self.enum_value(instruction, *enum_id, *variant, *layout, fields, values),
            InstructionKind::EnumIsVariant {
                enum_id,
                variant,
                layout,
                value: input,
            } => {
                let input_value = value(values, *input)?;
                if matches!(
                    input_value,
                    EvalValue::StructuralOwner(_) | EvalValue::StructuralView(_)
                ) {
                    let ty = function_value_type(function, *input)?;
                    self.structural_enum_is_variant(ty, *variant, *layout, input_value)
                } else {
                    match input_value {
                        EvalValue::Enum {
                            enum_id: actual,
                            variant: active,
                            layout: actual_layout,
                            ..
                        } if actual == enum_id && actual_layout == layout => {
                            Ok(EvalValue::Bool(active == variant))
                        }
                        _ => Err(Flow::Trap(
                            "enum variant test identity/layout mismatch".into(),
                        )),
                    }
                }
            }
            InstructionKind::EnumField {
                enum_id,
                variant,
                field,
                layout,
                value: input,
            } => self.enum_field(
                function, values, *input, *enum_id, *variant, *field, *layout,
            ),
            _ => Err(Flow::Trap("enum instruction dispatch mismatch".into())),
        }
    }

    fn enum_value(
        &mut self,
        instruction: &Instruction,
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        fields: &[ValueId],
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        match aggregate_mode(
            self.program.program(),
            self.config.structural_limits,
            &instruction.ty,
        )
        .map_err(Flow::Trap)?
        {
            AggregateMode::Structural => {
                self.enum_from_ssa(&instruction.ty, variant, layout, fields, values)
            }
            AggregateMode::Legacy | AggregateMode::ResourceAdapter => {
                let (selected, _, expected_layout) =
                    enum_variant(self.program.program(), &instruction.ty, variant)
                        .map_err(Flow::Trap)?;
                if expected_layout != layout {
                    return Err(Flow::Trap("enum construction layout mismatch".into()));
                }
                let physical_tag = selected.physical_tag;
                self.charge_aggregate()?;
                self.allocate()?;
                Ok(EvalValue::Enum {
                    enum_id,
                    variant,
                    layout,
                    physical_tag,
                    payload: values_for(values, fields)?,
                })
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn enum_field(
        &mut self,
        function: &crate::Function,
        values: &mut [Option<EvalValue>],
        input: ValueId,
        enum_id: EnumId,
        variant: VariantId,
        field: crate::VariantFieldId,
        layout: RuntimeLayoutId,
    ) -> Result<EvalValue, Flow> {
        let ty = function_value_type(function, input)?;
        let input_value = value(values, input)?;
        if matches!(
            input_value,
            EvalValue::StructuralOwner(_) | EvalValue::StructuralView(_)
        ) {
            return self.structural_enum_field(ty, variant, field, layout, input_value);
        }
        let (selected, _, _) =
            enum_variant(self.program.program(), ty, variant).map_err(Flow::Trap)?;
        let index = enum_field_index(selected, field).map_err(Flow::Trap)?;
        if aggregate_mode(self.program.program(), self.config.structural_limits, ty)
            .map_err(Flow::Trap)?
            == AggregateMode::ResourceAdapter
        {
            return self
                .consume_resource_adapter_field(values, input, enum_id, variant, layout, index);
        }
        match input_value {
            EvalValue::Enum {
                enum_id: actual,
                variant: active,
                layout: actual_layout,
                payload,
                ..
            } if *actual == enum_id && *active == variant && *actual_layout == layout => payload
                .get(index)
                .ok_or_else(|| Flow::Trap("enum active payload is malformed".into()))
                .and_then(clone_plain_eval_value),
            EvalValue::Enum { .. } => Err(Flow::Trap("inactive enum projection".into())),
            _ => Err(Flow::Trap("enum projection expects enum".into())),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn consume_resource_adapter_field(
        &mut self,
        values: &mut [Option<EvalValue>],
        input: ValueId,
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        index: usize,
    ) -> Result<EvalValue, Flow> {
        let owned = take_value(values, input)?;
        let EvalValue::Enum {
            enum_id: actual,
            variant: active,
            layout: actual_layout,
            mut payload,
            ..
        } = owned
        else {
            return Err(Flow::Trap("resource result projection expects enum".into()));
        };
        if actual != enum_id
            || active != variant
            || actual_layout != layout
            || index >= payload.len()
        {
            self.execute_unentered_argument_cleanup(payload);
            return Err(Flow::Trap("resource result projection mismatch".into()));
        }
        let field = payload.remove(index);
        if let Err(cleanup) = self.cleanup_legacy_values_reverse(payload) {
            self.note_structural_cleanup_failure(cleanup.detail());
        }
        Ok(field)
    }
}
