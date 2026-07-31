use lkjscript_core::Value as CoreValue;

use super::*;

impl Evaluator<'_> {
    pub(super) fn destination_instruction(
        &mut self,
        instruction: &Instruction,
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        match &instruction.kind {
            InstructionKind::DestinationCreate {
                representation,
                active_variant,
            } => self.create_destination(*representation, *active_variant),
            InstructionKind::DestinationFieldInit {
                destination,
                field,
                value: source,
            } => self.initialize_destination_field(values, *destination, *field, *source),
            InstructionKind::DestinationFinish { destination } => {
                self.finish_explicit_destination(values, *destination, &instruction.ty)
            }
            InstructionKind::DestinationAbort { destination } => {
                let destination = take_value(values, *destination)?;
                let EvalValue::StructuralDestination(destination) = destination else {
                    return Err(Flow::Trap("destination abort expects destination".into()));
                };
                self.structural
                    .runtime
                    .abort_destination(destination.key)
                    .map_err(map_structural_error)?;
                Ok(EvalValue::Unit)
            }
            _ => Err(Flow::Trap(
                "destination instruction dispatch mismatch".into(),
            )),
        }
    }

    fn create_destination(
        &mut self,
        representation: crate::StructuralRepresentationId,
        active_variant: Option<VariantId>,
    ) -> Result<EvalValue, Flow> {
        let facts =
            self.representation_facts(representation, crate::StructuralValueCategory::Destination)?;
        let value_type = self.structural_type(&facts.ty)?;
        let (tag, field_types) = destination_fields(&facts, active_variant)?;
        self.charge_aggregate()?;
        let fields = field_types
            .iter()
            .map(|field| self.structural_type(field))
            .collect::<Result<Vec<_>, _>>()?;
        self.allocate()?;
        let key = match tag {
            Some(tag) => self.structural.runtime.begin_enum(value_type, tag, fields),
            None if matches!(facts.layout, crate::StructuralLayoutKind::Product { .. }) => {
                self.structural.runtime.begin_product(value_type, fields)
            }
            None => {
                return Err(Flow::Trap(
                    "leaf structural destination is not constructible".into(),
                ))
            }
        }
        .map_err(map_structural_error)?;
        Ok(EvalValue::StructuralDestination(
            EvalStructuralDestination {
                key,
                value_type,
                type_id: facts.type_id,
                active_variant,
            },
        ))
    }

    fn initialize_destination_field(
        &mut self,
        values: &mut [Option<EvalValue>],
        destination_id: ValueId,
        field: u16,
        source: ValueId,
    ) -> Result<EvalValue, Flow> {
        let destination_value = take_value(values, destination_id)?;
        let EvalValue::StructuralDestination(destination) = destination_value else {
            return Err(Flow::Trap(
                "destination field init expects destination".into(),
            ));
        };
        let result = self.initialize_destination_source(&destination, field, values, source);
        match result {
            Ok(()) => Ok(EvalValue::StructuralDestination(destination)),
            Err(primary) => {
                if let Err(restore) = restore_slot(
                    values,
                    destination_id,
                    EvalValue::StructuralDestination(destination),
                ) {
                    self.note_structural_cleanup_failure(restore.detail());
                }
                Err(primary)
            }
        }
    }

    fn initialize_destination_source(
        &mut self,
        destination: &EvalStructuralDestination,
        field: u16,
        values: &mut [Option<EvalValue>],
        source: ValueId,
    ) -> Result<(), Flow> {
        let expected_ty = self.destination_field_type(destination, field)?;
        let expected = self.structural_type(&expected_ty)?;
        if let EvalValue::StructuralOwner(owner) = value(values, source)? {
            let owner = *owner;
            if owner.value_type != expected {
                return Err(Flow::Trap(
                    "destination structural field type mismatch".into(),
                ));
            }
            self.structural
                .runtime
                .initialize_value(
                    destination.key,
                    field,
                    CoreValue::from_structural_root(owner.key),
                )
                .map_err(map_structural_error)?;
            let removed = take_value(values, source)?;
            if !matches!(removed, EvalValue::StructuralOwner(actual) if actual == owner) {
                return Err(Flow::Trap(
                    "destination structural owner changed during transfer".into(),
                ));
            }
            return Ok(());
        }
        let transferred = matches!(
            value(values, source)?,
            EvalValue::Bytes(_) | EvalValue::ByteVector(_)
        );
        let semantic = if transferred {
            let owned = take_value(values, source)?;
            match self.take_semantic(owned, expected) {
                Ok(value) => value,
                Err(error) => {
                    let (flow, restored) = *error;
                    restore_slot(values, source, restored)?;
                    return Err(flow);
                }
            }
        } else {
            self.copy_semantic(value(values, source)?, expected)?
        };
        if let Err(failure) =
            self.structural
                .runtime
                .initialize_node(destination.key, field, semantic)
        {
            if transferred {
                let restored = self.semantic_to_eval(failure.value)?;
                restore_slot(values, source, restored)?;
            }
            return Err(map_structural_error(failure.error));
        }
        Ok(())
    }
}

include!("destination_finish.rs");
