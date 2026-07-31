use lkjscript_core::{SemanticPayload, StructuralFieldPath, StructuralKind, StructuralProjection};

use super::*;

impl Evaluator<'_> {
    pub(super) fn projection_instruction(
        &mut self,
        instruction: &Instruction,
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        match &instruction.kind {
            InstructionKind::AggregateTag {
                representation,
                value: source,
            } => self.aggregate_tag(values, *source, *representation),
            InstructionKind::AggregateFieldBorrow {
                representation,
                field,
                value: source,
                ..
            } => self.aggregate_field_borrow(instruction, values, *source, *representation, *field),
            InstructionKind::AggregateConsumePayload {
                representation,
                variant,
                value: source,
                ..
            } => self.consume_structural_payload(
                values,
                *source,
                *representation,
                *variant,
                &instruction.ty,
            ),
            InstructionKind::StringUtf8View {
                representation,
                value: source,
                ..
            } => self.string_utf8_view(values, *source, *representation),
            _ => Err(Flow::Trap(
                "projection instruction dispatch mismatch".into(),
            )),
        }
    }

    fn aggregate_tag(
        &self,
        values: &[Option<EvalValue>],
        source: ValueId,
        representation: crate::StructuralRepresentationId,
    ) -> Result<EvalValue, Flow> {
        let facts =
            self.representation_facts(representation, crate::StructuralValueCategory::View)?;
        let expected = self.structural_type(&facts.ty)?;
        let payload = self.explicit_structural_payload(value(values, source)?, expected)?;
        match payload.payload {
            SemanticPayload::Enum { tag, .. } => Ok(EvalValue::I64(i64::from(tag))),
            _ => Err(Flow::Trap("aggregate tag expects structural enum".into())),
        }
    }

    fn aggregate_field_borrow(
        &mut self,
        instruction: &Instruction,
        values: &[Option<EvalValue>],
        source: ValueId,
        representation: crate::StructuralRepresentationId,
        field: u16,
    ) -> Result<EvalValue, Flow> {
        let facts =
            self.representation_facts(representation, crate::StructuralValueCategory::View)?;
        let root_type = self.structural_type(&facts.ty)?;
        let expected_ty = aggregate_field_type(&facts, value(values, source)?, field)?;
        if expected_ty != instruction.ty {
            return Err(Flow::Trap("aggregate field borrow type mismatch".into()));
        }
        let expected = self.structural_type(&expected_ty)?;
        let owner = self.explicit_structural_owner(value(values, source)?, root_type)?;
        let key = self
            .structural
            .runtime
            .borrow_projected(
                owner,
                root_type,
                StructuralProjection::Field {
                    path: StructuralFieldPath::new(vec![field]),
                    expected,
                },
                false,
            )
            .map_err(map_structural_error)?;
        Ok(EvalValue::StructuralView(EvalStructuralView {
            owner,
            key,
            root_type,
            value_type: expected,
        }))
    }

    fn string_utf8_view(
        &mut self,
        values: &[Option<EvalValue>],
        source: ValueId,
        representation: crate::StructuralRepresentationId,
    ) -> Result<EvalValue, Flow> {
        let facts =
            self.representation_facts(representation, crate::StructuralValueCategory::View)?;
        let root_type = self.structural_type(&facts.ty)?;
        if root_type.kind != StructuralKind::String {
            return Err(Flow::Trap("UTF-8 view representation is not string".into()));
        }
        let input = value(values, source)?;
        let owner = self.explicit_structural_owner(input, root_type)?;
        let length = match self.explicit_structural_payload(input, root_type)?.payload {
            SemanticPayload::String(bytes) => {
                std::str::from_utf8(&bytes)
                    .map_err(|_| Flow::Trap("structural string is not UTF-8".into()))?;
                u32::try_from(bytes.len())
                    .map_err(|_| Flow::Resource("structural UTF-8 range".into()))?
            }
            _ => return Err(Flow::Trap("UTF-8 view expects string payload".into())),
        };
        let key = self
            .structural
            .runtime
            .borrow_projected(
                owner,
                root_type,
                StructuralProjection::Utf8 {
                    path: StructuralFieldPath::root(),
                    expected: root_type,
                    start: 0,
                    end: length,
                },
                false,
            )
            .map_err(map_structural_error)?;
        Ok(EvalValue::StructuralUtf8View(EvalStructuralView {
            owner,
            key,
            root_type,
            value_type: root_type,
        }))
    }
}

include!("projection_consume.rs");
