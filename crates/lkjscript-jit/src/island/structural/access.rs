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
            return Err(NativeServiceError::HostFailure);
        }
        fields.extend(projection.path().iter().copied().map(usize::from));
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
        if let Err(error) = self.enforce_policy() {
            let _ = self.runtime.end_view(key);
            return Err(error);
        }
        Ok(NativeStructuralView::new(view_type, key.get()))
    }

    pub(super) fn borrow_utf8(
        &mut self,
        owner: NativeStructuralOwner,
        projection: &StructuralProjectionDescriptor,
    ) -> Result<NativeStructuralView, NativeServiceError> {
        let expected = core_type(owner.structural_type())?;
        let length = {
            let node = match self.runtime.value_node(owner_key(owner)?, expected) {
                Ok(node) => node,
                Err(error) => return Err(self.map_error(error)),
            };
            let bytes = node_bytes(node)?;
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
        let node = match self.runtime.projected_node(key) {
            Ok(node) => node,
            Err(error) => return Err(self.map_error(error)),
        };
        let StructuralNodeView::Enum { tag, .. } = node.payload() else {
            return Err(NativeServiceError::Trap);
        };
        i64::try_from(tag).map_err(|_| NativeServiceError::Trap)
    }

    pub(super) fn owned_tag(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<i64, NativeServiceError> {
        self.note_call();
        let expected = core_type(owner.structural_type())?;
        let node = match self.runtime.value_node(owner_key(owner)?, expected) {
            Ok(node) => node,
            Err(error) => return Err(self.map_error(error)),
        };
        let StructuralNodeView::Enum { tag, .. } = node.payload() else {
            return Err(NativeServiceError::Trap);
        };
        i64::try_from(tag).map_err(|_| NativeServiceError::Trap)
    }

    pub(super) fn length(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<i64, NativeServiceError> {
        self.note_call();
        let expected = core_type(owner.structural_type())?;
        let node = match self.runtime.value_node(owner_key(owner)?, expected) {
            Ok(node) => node,
            Err(error) => return Err(self.map_error(error)),
        };
        i64::try_from(node_bytes(node)?.len()).map_err(|_| {
            self.last_resource = Some(ResourceLimitKind::HeapBytes);
            NativeServiceError::ResourceLimitExceeded
        })
    }

    pub(super) fn i64(&mut self, view: NativeStructuralView) -> Result<i64, NativeServiceError> {
        self.note_call();
        let key = view_key(view)?;
        let node = match self.runtime.projected_node(key) {
            Ok(node) => node,
            Err(error) => return Err(self.map_error(error)),
        };
        let StructuralNodeView::Inline(InlineStructuralValue::I64(value)) = node.payload() else {
            return Err(NativeServiceError::Trap);
        };
        Ok(value)
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
            self.runtime.projected_node(left_key),
            self.runtime.projected_node(right_key),
        ) {
            (Ok(left), Ok(right)) => (left, right),
            (Err(error), _) | (_, Err(error)) => return Err(self.map_error(error)),
        };
        Ok(node_bytes(pair.0)? == node_bytes(pair.1)?)
    }

    pub(super) fn utf8_valid(
        &mut self,
        view: NativeStructuralView,
    ) -> Result<bool, NativeServiceError> {
        self.note_call();
        let key = view_key(view)?;
        let node = match self.runtime.projected_node(key) {
            Ok(node) => node,
            Err(error) => return Err(self.map_error(error)),
        };
        Ok(std::str::from_utf8(node_bytes(node)?).is_ok())
    }
}
