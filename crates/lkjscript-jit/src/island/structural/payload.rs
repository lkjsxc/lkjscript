use super::*;

impl JitStructuralRuntime {
    pub(super) fn copy_view(
        &mut self,
        view: NativeStructuralView,
    ) -> Result<NativeValue, NativeServiceError> {
        self.note_call();
        let expected = view.view_type().projected();
        let semantic = match self.runtime.projected(view_key(view)?) {
            Ok(value) => value,
            Err(error) => return Err(self.map_error(error)),
        };
        self.payload_value(semantic, expected)
    }

    pub(super) fn consume_payload(
        &mut self,
        owner: NativeStructuralOwner,
        aggregate: &StructuralAggregateDescriptor,
    ) -> Result<NativeValue, NativeServiceError> {
        self.note_call();
        if owner.structural_type() != aggregate.value_type() || aggregate.fields().len() != 1 {
            return Err(NativeServiceError::Trap);
        }
        let StructuralAggregateKind::Enum(expected_tag) = aggregate.kind() else {
            return Err(NativeServiceError::Trap);
        };
        let owner_type = core_type(owner.structural_type())?;
        let payload_type = core_type(aggregate.fields()[0])?;
        let owner_key = owner_key(owner)?;
        let node = match self.runtime.value_node(owner_key, owner_type) {
            Ok(node) => node,
            Err(error) => return Err(self.map_error(error)),
        };
        match node.payload() {
            StructuralNodeView::Enum { tag, fields }
                if tag == u64::from(expected_tag)
                    && fields.len() == 1
                    && node
                        .child(0)
                        .is_some_and(|field| field.value_type() == payload_type) => {}
            StructuralNodeView::Enum { .. } => return Err(NativeServiceError::Trap),
            _ => return Err(NativeServiceError::Trap),
        }
        let semantic = self
            .runtime
            .export_semantic(owner_key, owner_type)
            .map_err(|error| self.map_error(error))?;
        self.owners.remove(&owner_key.get());
        let SemanticPayload::Enum {
            mut active_payload, ..
        } = semantic.payload
        else {
            return Err(NativeServiceError::HostFailure);
        };
        let payload = active_payload
            .pop()
            .ok_or(NativeServiceError::HostFailure)?;
        self.payload_value(payload, aggregate.fields()[0])
    }

    fn payload_value(
        &mut self,
        value: SemanticValue,
        expected: StructuralTypeIdentity,
    ) -> Result<NativeValue, NativeServiceError> {
        let expected_type = core_type(expected)?;
        if value.value_type != expected_type {
            return Err(NativeServiceError::HostFailure);
        }
        match value.payload {
            SemanticPayload::Inline(InlineStructuralValue::Unit) => Ok(NativeValue::Unit),
            SemanticPayload::Inline(InlineStructuralValue::Bool(value)) => {
                Ok(NativeValue::Bool(value))
            }
            SemanticPayload::Inline(InlineStructuralValue::I64(value)) => {
                Ok(NativeValue::I64(value))
            }
            SemanticPayload::Inline(InlineStructuralValue::F64Bits(bits)) => {
                Ok(NativeValue::F64Bits(bits))
            }
            payload => {
                let value = SemanticValue::new(expected_type, payload);
                let key = self
                    .runtime
                    .publish_owned(value)
                    .map_err(|failure| self.map_error(failure.error))?;
                self.register_runtime_owner(
                    key,
                    expected_type,
                    expected,
                    StructuralStorageRoute::Unique,
                )?;
                Ok(NativeValue::StructuralOwner(NativeStructuralOwner::new(
                    expected,
                    key.get(),
                )))
            }
        }
    }
}
