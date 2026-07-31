use lkjscript_core::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralFieldPath, StructuralKind,
    StructuralNodeView, StructuralProjection, StructuralType, StructuralValueError,
    StructuralValueKey,
};

use crate::eval::{EvalStructuralOwner, EvalValue, Evaluator, Flow};

impl Evaluator<'_> {
    pub(crate) fn structural_string(&mut self, text: String) -> Result<EvalValue, Flow> {
        self.allocate_dynamic(text.capacity())?;
        let value_type = self.structural_type(&crate::SsaType::Str)?;
        self.publish_structural(SemanticValue::new(
            value_type,
            SemanticPayload::String(text.into_bytes()),
        ))
    }

    pub(crate) fn structural_path(&mut self, bytes: Vec<u8>) -> Result<EvalValue, Flow> {
        self.allocate_dynamic(bytes.capacity())?;
        let value_type = self.structural_type(&crate::SsaType::Path)?;
        self.publish_structural(SemanticValue::new(value_type, SemanticPayload::Path(bytes)))
    }

    pub(crate) fn publish_structural(&mut self, value: SemanticValue) -> Result<EvalValue, Flow> {
        let value_type = value.value_type;
        self.structural
            .runtime
            .publish_owned(value)
            .map(|key| EvalValue::StructuralOwner(EvalStructuralOwner { key, value_type }))
            .map_err(|failure| map_structural_error(failure.error))
    }

    pub(crate) fn structural_type(&self, ty: &crate::SsaType) -> Result<StructuralType, Flow> {
        super::structural_type(self.program.program(), ty).map_err(Flow::Trap)
    }

    pub(crate) fn string_bytes_copy(&mut self, value: &EvalValue) -> Result<Vec<u8>, Flow> {
        match value {
            EvalValue::Str(text) => return copy_bytes(text.as_bytes()),
            EvalValue::StaticString(identity) => {
                return copy_bytes(
                    self.structural
                        .static_string(*identity)
                        .map_err(Flow::Trap)?
                        .as_bytes(),
                )
            }
            _ => {}
        }
        let (owner, value_type) = structural_root(value, StructuralKind::String)?;
        let length = match self
            .structural
            .runtime
            .value_node(owner, value_type)
            .map_err(map_structural_error)?
            .payload()
        {
            StructuralNodeView::Bytes(bytes) => bytes.len(),
            _ => return Err(Flow::Trap("expected structural string payload".into())),
        };
        let end = u32::try_from(length)
            .map_err(|_| Flow::Resource("structural string view range".into()))?;
        let view = self
            .structural
            .runtime
            .borrow_projected(
                owner,
                value_type,
                StructuralProjection::Utf8 {
                    path: StructuralFieldPath::root(),
                    expected: value_type,
                    start: 0,
                    end,
                },
                false,
            )
            .map_err(map_structural_error)?;
        let result = self
            .structural
            .runtime
            .utf8_view(view)
            .map_err(map_structural_error)
            .and_then(|text| copy_bytes(text.as_bytes()));
        let ended = self
            .structural
            .runtime
            .end_view(view)
            .map_err(map_structural_error);
        match (result, ended) {
            (Ok(bytes), Ok(())) => Ok(bytes),
            (Err(primary), _) => Err(primary),
            (Ok(_), Err(cleanup)) => Err(cleanup),
        }
    }

    pub(crate) fn string_text_copy(&mut self, value: &EvalValue) -> Result<String, Flow> {
        String::from_utf8(self.string_bytes_copy(value)?)
            .map_err(|_| Flow::Trap("invalid UTF-8 structural string".into()))
    }

    pub(crate) fn path_bytes_copy(&mut self, value: &EvalValue) -> Result<Vec<u8>, Flow> {
        if matches!(value, EvalValue::Path(_)) {
            return self.unique.copy_path(value);
        }
        let (owner, value_type) = structural_root(value, StructuralKind::Path)?;
        let view = self.borrow_whole(owner, value_type)?;
        let result = self
            .structural
            .runtime
            .projected_node(view)
            .map_err(map_structural_error)
            .and_then(|node| match node.payload() {
                StructuralNodeView::Bytes(bytes) => copy_bytes(bytes),
                _ => Err(Flow::Trap("expected structural path payload".into())),
            });
        let ended = self
            .structural
            .runtime
            .end_view(view)
            .map_err(map_structural_error);
        match (result, ended) {
            (Ok(bytes), Ok(())) => Ok(bytes),
            (Err(primary), _) => Err(primary),
            (Ok(_), Err(cleanup)) => Err(cleanup),
        }
    }

    pub(crate) fn borrow_whole(
        &mut self,
        owner: StructuralValueKey,
        value_type: StructuralType,
    ) -> Result<lkjscript_core::StructuralViewKey, Flow> {
        self.structural
            .runtime
            .borrow_projected(
                owner,
                value_type,
                StructuralProjection::Field {
                    path: StructuralFieldPath::root(),
                    expected: value_type,
                },
                false,
            )
            .map_err(map_structural_error)
    }
}

pub(crate) fn structural_root(
    value: &EvalValue,
    kind: StructuralKind,
) -> Result<(StructuralValueKey, StructuralType), Flow> {
    let (key, value_type) = match value {
        EvalValue::StructuralOwner(owner) => (owner.key, owner.value_type),
        EvalValue::StructuralView(view) => (view.owner, view.value_type),
        _ => return Err(Flow::Trap(format!("expected structural {kind:?} value"))),
    };
    if value_type.kind != kind {
        return Err(Flow::Trap("structural value kind mismatch".into()));
    }
    Ok((key, value_type))
}

pub(crate) fn map_structural_error(error: StructuralValueError) -> Flow {
    match error {
        StructuralValueError::AllocationFailed | StructuralValueError::LimitExceeded(_) => {
            Flow::Resource(format!("structural value: {error}"))
        }
        _ => Flow::Trap(error.to_string()),
    }
}

pub(crate) fn inline_eval(value: InlineStructuralValue) -> EvalValue {
    match value {
        InlineStructuralValue::Unit => EvalValue::Unit,
        InlineStructuralValue::Bool(value) => EvalValue::Bool(value),
        InlineStructuralValue::I64(value) => EvalValue::I64(value),
        InlineStructuralValue::F64Bits(value) => EvalValue::F64(f64::from_bits(value)),
    }
}

fn copy_bytes(bytes: &[u8]) -> Result<Vec<u8>, Flow> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| Flow::Resource("structural payload bytes".into()))?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}
