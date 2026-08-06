use lkjscript_core::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StaticStructuralLeaf, StructuralKind,
    StructuralType, UniqueLayout,
};

use crate::eval::{
    inline_eval, map_structural_error, structural_root, EvalStructuralOwner, EvalValue, Evaluator,
    Flow,
};

impl Evaluator<'_> {
    pub(crate) fn copy_eval_value(&mut self, value: &EvalValue) -> Result<EvalValue, Flow> {
        match value {
            EvalValue::StructuralOwner(owner) => self.clone_structural(owner.key, owner.value_type),
            EvalValue::StructuralView(view) => {
                let semantic = self
                    .structural
                    .runtime
                    .projected(view.key)
                    .map_err(map_structural_error)?;
                self.semantic_to_eval(semantic)
            }
            EvalValue::Bytes(_) => self.unique.clone_bytes(value),
            EvalValue::Path(_) => self.unique.clone_path(value),
            EvalValue::StructuralUtf8View(_)
            | EvalValue::StructuralDestination(_)
            | EvalValue::ByteVector(_)
            | EvalValue::BytesBorrow(_)
            | EvalValue::ByteSlice(_)
            | EvalValue::ByteSliceMut(_)
            | EvalValue::Resource(_) => {
                Err(Flow::Trap("affine evaluator value cannot be copied".into()))
            }
            other => clone_plain_eval_value(other),
        }
    }

    fn clone_structural(
        &mut self,
        key: lkjscript_core::StructuralValueKey,
        value_type: StructuralType,
    ) -> Result<EvalValue, Flow> {
        self.structural
            .runtime
            .independent_owner(key, value_type)
            .map(|key| EvalValue::StructuralOwner(EvalStructuralOwner { key, value_type }))
            .map_err(map_structural_error)
    }

    pub(crate) fn copy_semantic(
        &mut self,
        value: &EvalValue,
        expected: StructuralType,
    ) -> Result<SemanticValue, Flow> {
        match value {
            EvalValue::Unit => Ok(inline(expected, InlineStructuralValue::Unit)),
            EvalValue::Bool(value) => Ok(inline(expected, InlineStructuralValue::Bool(*value))),
            EvalValue::I64(value) => Ok(inline(expected, InlineStructuralValue::I64(*value))),
            EvalValue::F64(value) => Ok(inline(
                expected,
                InlineStructuralValue::F64Bits(value.to_bits()),
            )),
            EvalValue::StaticString(identity) => {
                let bytes = copy_bytes(
                    self.structural
                        .static_string(*identity)
                        .map_err(Flow::Trap)?
                        .as_bytes(),
                )?;
                Ok(SemanticValue::new(expected, SemanticPayload::String(bytes)))
            }
            EvalValue::StaticSymbol(identity) => Ok(SemanticValue::new(
                expected,
                SemanticPayload::Static(StaticStructuralLeaf::Symbol(*identity)),
            )),
            EvalValue::Symbol(symbol) => {
                let identity = self
                    .structural
                    .static_symbol_identity(symbol)
                    .map_err(Flow::Trap)?;
                Ok(SemanticValue::new(
                    expected,
                    SemanticPayload::Static(StaticStructuralLeaf::Symbol(identity)),
                ))
            }
            EvalValue::Function(function) => Ok(SemanticValue::new(
                expected,
                SemanticPayload::Static(StaticStructuralLeaf::Function(function.raw())),
            )),
            EvalValue::Path(_) if expected.kind == StructuralKind::Path => Ok(SemanticValue::new(
                expected,
                SemanticPayload::Path(self.unique.copy_path(value)?),
            )),
            EvalValue::StructuralOwner(_) | EvalValue::StructuralView(_) => {
                let (key, actual) = structural_root(value, expected.kind)?;
                if actual != expected {
                    return Err(Flow::Trap("structural field type mismatch".into()));
                }
                let clone = self
                    .structural
                    .runtime
                    .independent_owner(key, expected)
                    .map_err(map_structural_error)?;
                self.structural
                    .runtime
                    .export_semantic(clone, expected)
                    .map_err(map_structural_error)
            }
            _ => Err(Flow::Trap(
                "value cannot be copied into a structural destination".into(),
            )),
        }
    }

    pub(crate) fn take_semantic(
        &mut self,
        value: EvalValue,
        expected: StructuralType,
    ) -> Result<SemanticValue, Box<(Flow, EvalValue)>> {
        match value {
            EvalValue::StructuralOwner(owner) if owner.value_type == expected => self
                .structural
                .runtime
                .export_semantic(owner.key, expected)
                .map_err(map_structural_error)
                .map_err(|flow| Box::new((flow, EvalValue::StructuralOwner(owner)))),
            EvalValue::Bytes(_) if expected.kind == StructuralKind::Bytes => {
                self.take_unique_semantic(value, expected, UniqueLayout::Bytes)
            }
            EvalValue::ByteVector(_) if expected.kind == StructuralKind::ByteVector => {
                self.take_unique_semantic(value, expected, UniqueLayout::ByteVector)
            }
            EvalValue::Path(_) if expected.kind == StructuralKind::Path => {
                self.take_unique_semantic(value, expected, UniqueLayout::Path)
            }
            other => self
                .copy_semantic(&other, expected)
                .map_err(|flow| Box::new((flow, other))),
        }
    }

    fn take_unique_semantic(
        &mut self,
        value: EvalValue,
        expected: StructuralType,
        layout: UniqueLayout,
    ) -> Result<SemanticValue, Box<(Flow, EvalValue)>> {
        let original = match &value {
            EvalValue::Bytes(word) => EvalValue::Bytes(*word),
            EvalValue::ByteVector(word) => EvalValue::ByteVector(*word),
            EvalValue::Path(word) => EvalValue::Path(*word),
            _ => {
                return Err(Box::new((
                    Flow::Trap("expected unique structural leaf".into()),
                    value,
                )))
            }
        };
        let bytes = self
            .unique
            .export_owner(value)
            .map_err(|flow| Box::new((flow, original)))?;
        let payload = match layout {
            UniqueLayout::Bytes => SemanticPayload::Bytes(bytes),
            UniqueLayout::ByteVector => SemanticPayload::ByteVector(bytes),
            UniqueLayout::Path => SemanticPayload::Path(bytes),
        };
        Ok(SemanticValue::new(expected, payload))
    }

    pub(crate) fn semantic_to_eval(&mut self, value: SemanticValue) -> Result<EvalValue, Flow> {
        match value.payload {
            SemanticPayload::Inline(value) => Ok(inline_eval(value)),
            SemanticPayload::Static(StaticStructuralLeaf::Function(id)) => {
                Ok(EvalValue::Function(crate::FunctionId::new(id)))
            }
            SemanticPayload::Static(StaticStructuralLeaf::Symbol(id)) => {
                self.structural.static_symbol(id).map_err(Flow::Trap)?;
                Ok(EvalValue::StaticSymbol(id))
            }
            SemanticPayload::Static(StaticStructuralLeaf::Bytes(id)) => {
                Ok(EvalValue::StaticBytes(id))
            }
            SemanticPayload::Bytes(bytes) => self.unique.restore_owner(bytes, UniqueLayout::Bytes),
            SemanticPayload::ByteVector(bytes) => {
                self.unique.restore_owner(bytes, UniqueLayout::ByteVector)
            }
            SemanticPayload::Path(bytes) => self.unique.restore_owner(bytes, UniqueLayout::Path),
            payload => self.publish_structural(SemanticValue::new(value.value_type, payload)),
        }
    }
}

fn inline(value_type: StructuralType, value: InlineStructuralValue) -> SemanticValue {
    SemanticValue::new(value_type, SemanticPayload::Inline(value))
}

mod plain;
pub(crate) use plain::{clone_plain_eval_value, copy_bytes};
