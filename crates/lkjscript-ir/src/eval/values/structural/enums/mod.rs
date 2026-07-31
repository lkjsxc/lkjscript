use crate::eval::{
    map_structural_error, take_value, value, EvalStructuralOwner, EvalValue, Evaluator, Flow,
};
use crate::{RuntimeLayoutId, SsaType, ValueId, VariantId};

impl Evaluator<'_> {
    pub(crate) fn enum_from_ssa(
        &mut self,
        ty: &SsaType,
        variant: VariantId,
        layout: RuntimeLayoutId,
        fields: &[ValueId],
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        let (selected, field_types, expected_layout) =
            super::enum_variant(self.program.program(), ty, variant).map_err(Flow::Trap)?;
        if expected_layout != layout || fields.len() != field_types.len() {
            return Err(Flow::Trap("enum construction metadata mismatch".into()));
        }
        let physical_tag = selected.physical_tag;
        self.charge_aggregate()?;
        self.allocate()?;
        let value_type = self.structural_type(ty)?;
        let structural_fields = field_types
            .iter()
            .map(|field| self.structural_type(field))
            .collect::<Result<Vec<_>, _>>()?;
        let destination = self
            .structural
            .runtime
            .begin_enum(value_type, physical_tag, structural_fields.clone())
            .map_err(map_structural_error)?;
        for (index, ((source, ty), expected)) in fields
            .iter()
            .zip(&field_types)
            .zip(structural_fields)
            .enumerate()
        {
            let transferred = matches!(ty, SsaType::Bytes | SsaType::ByteVector);
            let semantic = if transferred {
                let owned = take_value(values, *source)?;
                match self.take_semantic(owned, expected) {
                    Ok(value) => value,
                    Err(error) => {
                        let (flow, restored) = *error;
                        super::restore_slot(values, *source, restored)?;
                        self.abort_structural_destination(destination);
                        return Err(flow);
                    }
                }
            } else {
                self.copy_semantic(value(values, *source)?, expected)?
            };
            if let Err(failure) = self.structural.runtime.initialize_node(
                destination,
                u16::try_from(index)
                    .map_err(|_| Flow::Resource("structural enum fields".into()))?,
                semantic,
            ) {
                if transferred {
                    let restored = self.semantic_to_eval(failure.value)?;
                    super::restore_slot(values, *source, restored)?;
                }
                self.abort_structural_destination(destination);
                return Err(map_structural_error(failure.error));
            }
        }
        match self.structural.runtime.finish_destination(destination) {
            Ok(key) => Ok(EvalValue::StructuralOwner(EvalStructuralOwner {
                key,
                value_type,
            })),
            Err(error) => {
                self.abort_structural_destination(destination);
                Err(map_structural_error(error))
            }
        }
    }

    pub(crate) fn construct_enum(
        &mut self,
        ty: &SsaType,
        variant: VariantId,
        mut payload: Vec<EvalValue>,
    ) -> Result<EvalValue, Flow> {
        let mode = super::aggregate_mode(self.program.program(), self.config.structural_limits, ty)
            .map_err(Flow::Trap)?;
        let (selected, fields, layout) =
            super::enum_variant(self.program.program(), ty, variant).map_err(Flow::Trap)?;
        if payload.len() != fields.len() {
            self.execute_unentered_argument_cleanup(payload);
            return Err(Flow::Trap("enum payload shape mismatch".into()));
        }
        if mode != super::AggregateMode::Structural {
            self.charge_aggregate()?;
            self.allocate()?;
            let SsaType::Enum { id, .. } = ty else {
                return Err(Flow::Trap("expected enum type".into()));
            };
            return Ok(EvalValue::Enum {
                enum_id: *id,
                variant,
                layout,
                physical_tag: selected.physical_tag,
                payload,
            });
        }
        self.charge_aggregate()?;
        self.allocate()?;
        let value_type = self.structural_type(ty)?;
        let structural_fields = fields
            .iter()
            .map(|field| self.structural_type(field))
            .collect::<Result<Vec<_>, _>>()?;
        let destination = self
            .structural
            .runtime
            .begin_enum(value_type, selected.physical_tag, structural_fields.clone())
            .map_err(map_structural_error)?;
        for (index, expected) in structural_fields.into_iter().enumerate() {
            let owned = payload.remove(0);
            let semantic = match self.take_semantic(owned, expected) {
                Ok(value) => value,
                Err(error) => {
                    let (flow, restored) = *error;
                    payload.insert(0, restored);
                    self.abort_structural_destination(destination);
                    self.execute_unentered_argument_cleanup(payload);
                    return Err(flow);
                }
            };
            if let Err(failure) = self.structural.runtime.initialize_node(
                destination,
                u16::try_from(index)
                    .map_err(|_| Flow::Resource("structural enum fields".into()))?,
                semantic,
            ) {
                let restored = self.semantic_to_eval(failure.value)?;
                payload.insert(0, restored);
                self.abort_structural_destination(destination);
                self.execute_unentered_argument_cleanup(payload);
                return Err(map_structural_error(failure.error));
            }
        }
        match self.structural.runtime.finish_destination(destination) {
            Ok(key) => Ok(EvalValue::StructuralOwner(EvalStructuralOwner {
                key,
                value_type,
            })),
            Err(error) => {
                self.abort_structural_destination(destination);
                Err(map_structural_error(error))
            }
        }
    }
}

mod query;
