use super::*;

impl JitStructuralRuntime {
    pub(super) fn borrow(
        &mut self,
        owner: NativeStructuralOwner,
        projection: &StructuralProjectionDescriptor,
        start: i64,
        end: i64,
    ) -> Result<NativeStructuralView, NativeServiceError> {
        self.note_call();
        let view_type = projection.view_type();
        if owner.structural_type() != view_type.root() {
            return Err(NativeServiceError::Trap);
        }
        let root_type = core_type(view_type.root())?;
        let expected = core_type(view_type.projected())?;
        let mut fields = Vec::new();
        if fields.try_reserve_exact(projection.path().len()).is_err() {
            self.last_resource = Some(ResourceLimitKind::HeapBytes);
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        fields.extend_from_slice(projection.path());
        let path = StructuralFieldPath::new(fields);
        let projection = match projection.kind() {
            StructuralProjectionKind::Field => StructuralProjection::Field { path, expected },
            StructuralProjectionKind::Utf8 => StructuralProjection::Utf8 {
                path,
                expected,
                start: u32::try_from(start).map_err(|_| NativeServiceError::Trap)?,
                end: u32::try_from(end).map_err(|_| NativeServiceError::Trap)?,
            },
        };
        let key = self
            .runtime
            .borrow_projected(
                owner_key(owner)?,
                root_type,
                projection,
                view_type.exclusive(),
            )
            .map_err(|error| self.map_error(error))?;
        Ok(NativeStructuralView::new(view_type, key.get()))
    }

    pub(super) fn borrow_utf8(
        &mut self,
        owner: NativeStructuralOwner,
        projection: &StructuralProjectionDescriptor,
    ) -> Result<NativeStructuralView, NativeServiceError> {
        let expected = core_type(owner.structural_type())?;
        let length = {
            let value = match self.runtime.value(owner_key(owner)?, expected) {
                Ok(value) => value,
                Err(error) => return Err(self.map_error(error)),
            };
            let SemanticPayload::String(bytes) = &value.payload else {
                return Err(NativeServiceError::Trap);
            };
            std::str::from_utf8(bytes).map_err(|_| NativeServiceError::Trap)?;
            i64::try_from(bytes.len()).map_err(|_| NativeServiceError::ResourceLimitExceeded)?
        };
        self.borrow(owner, projection, 0, length)
    }

    pub(super) fn end_view(
        &mut self,
        view: NativeStructuralView,
    ) -> Result<(), NativeServiceError> {
        self.note_call();
        self.runtime
            .end_view(view_key(view)?)
            .map_err(|error| self.map_error(error))
    }

    pub(super) fn tag(&mut self, view: NativeStructuralView) -> Result<i64, NativeServiceError> {
        self.note_call();
        let key = view_key(view)?;
        let value = match self.runtime.projected(key) {
            Ok(value) => value,
            Err(error) => return Err(self.map_error(error)),
        };
        let SemanticPayload::Enum { tag, .. } = &value.payload else {
            return Err(NativeServiceError::Trap);
        };
        Ok(i64::from(*tag))
    }

    pub(super) fn owned_tag(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<i64, NativeServiceError> {
        self.note_call();
        let expected = core_type(owner.structural_type())?;
        let value = match self.runtime.value(owner_key(owner)?, expected) {
            Ok(value) => value,
            Err(error) => return Err(self.map_error(error)),
        };
        let SemanticPayload::Enum { tag, .. } = &value.payload else {
            return Err(NativeServiceError::Trap);
        };
        Ok(i64::from(*tag))
    }

    pub(super) fn length(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<i64, NativeServiceError> {
        self.note_call();
        let expected = core_type(owner.structural_type())?;
        let value = match self.runtime.value(owner_key(owner)?, expected) {
            Ok(value) => value,
            Err(error) => return Err(self.map_error(error)),
        };
        i64::try_from(bytes(value)?.len()).map_err(|_| {
            self.last_resource = Some(ResourceLimitKind::HeapBytes);
            NativeServiceError::ResourceLimitExceeded
        })
    }

    pub(super) fn i64(&mut self, view: NativeStructuralView) -> Result<i64, NativeServiceError> {
        self.note_call();
        let key = view_key(view)?;
        let value = match self.runtime.projected(key) {
            Ok(value) => value,
            Err(error) => return Err(self.map_error(error)),
        };
        let SemanticPayload::Inline(InlineStructuralValue::I64(value)) = &value.payload else {
            return Err(NativeServiceError::Trap);
        };
        Ok(*value)
    }

    pub(super) fn bytes_equal(
        &mut self,
        left: NativeStructuralView,
        right: NativeStructuralView,
    ) -> Result<bool, NativeServiceError> {
        self.note_call();
        let left_key = view_key(left)?;
        let right_key = view_key(right)?;
        let pair = match (
            self.runtime.projected(left_key),
            self.runtime.projected(right_key),
        ) {
            (Ok(left), Ok(right)) => (left, right),
            (Err(error), _) | (_, Err(error)) => return Err(self.map_error(error)),
        };
        Ok(bytes(pair.0)? == bytes(pair.1)?)
    }

    pub(super) fn utf8_valid(
        &mut self,
        view: NativeStructuralView,
    ) -> Result<bool, NativeServiceError> {
        self.note_call();
        let key = view_key(view)?;
        let value = match self.runtime.projected(key) {
            Ok(value) => value,
            Err(error) => return Err(self.map_error(error)),
        };
        Ok(std::str::from_utf8(bytes(value)?).is_ok())
    }
}
