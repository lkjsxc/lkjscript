use lkjscript_core::{SemanticPayload, StructuralKind};

use crate::eval::{
    enum_field_index, enum_variant, map_structural_error, structural_root, EvalValue, Evaluator,
    Flow,
};
use crate::{RuntimeLayoutId, SsaType, VariantFieldId, VariantId};

impl Evaluator<'_> {
    pub(crate) fn structural_enum_is_variant(
        &mut self,
        ty: &SsaType,
        variant: VariantId,
        layout: RuntimeLayoutId,
        input: &EvalValue,
    ) -> Result<EvalValue, Flow> {
        let (selected, _, expected_layout) =
            enum_variant(self.program.program(), ty, variant).map_err(Flow::Trap)?;
        if expected_layout != layout {
            return Err(Flow::Trap("enum variant test layout mismatch".into()));
        }
        let value_type = self.structural_type(ty)?;
        let (owner, actual) = structural_root(input, StructuralKind::Enum)?;
        if actual != value_type {
            return Err(Flow::Trap("enum variant test type mismatch".into()));
        }
        let view = self.borrow_whole(owner, value_type)?;
        let result = match &self
            .structural
            .runtime
            .projected(view)
            .map_err(map_structural_error)?
            .payload
        {
            SemanticPayload::Enum { tag, .. } => Ok(EvalValue::Bool(*tag == selected.physical_tag)),
            _ => Err(Flow::Trap("expected structural enum payload".into())),
        };
        let ended = self
            .structural
            .runtime
            .end_view(view)
            .map_err(map_structural_error);
        match (result, ended) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(primary), _) => Err(primary),
            (Ok(_), Err(cleanup)) => Err(cleanup),
        }
    }

    pub(crate) fn structural_enum_field(
        &mut self,
        ty: &SsaType,
        variant: VariantId,
        field: VariantFieldId,
        layout: RuntimeLayoutId,
        input: &EvalValue,
    ) -> Result<EvalValue, Flow> {
        let (selected, fields, expected_layout) =
            enum_variant(self.program.program(), ty, variant).map_err(Flow::Trap)?;
        if expected_layout != layout {
            return Err(Flow::Trap("enum field layout mismatch".into()));
        }
        let index = enum_field_index(selected, field).map_err(Flow::Trap)?;
        let field_ty = fields
            .get(index)
            .ok_or_else(|| Flow::Trap("enum field metadata mismatch".into()))?;
        if *field_ty == SsaType::ByteVector {
            return Err(Flow::Trap(
                "affine enum projection requires consuming metadata".into(),
            ));
        }
        let value_type = self.structural_type(ty)?;
        let (owner, actual) = structural_root(input, StructuralKind::Enum)?;
        if actual != value_type {
            return Err(Flow::Trap("enum projection type mismatch".into()));
        }
        if !matches!(
            self.structural
                .runtime
                .value(owner, value_type)
                .map_err(map_structural_error)?
                .payload,
            SemanticPayload::Enum { tag, .. } if tag == selected.physical_tag
        ) {
            return Err(Flow::Trap("inactive enum projection".into()));
        }
        let expected = self.structural_type(field_ty)?;
        let semantic = self.projected_semantic(owner, value_type, index, expected)?;
        self.semantic_to_eval(semantic)
    }
}
