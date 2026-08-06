use super::super::*;

impl<J: RuntimeTier> Vm<'_, J> {
    pub(crate) fn list_prepend(&mut self, head: Value, tail: Value) -> Result<Value> {
        let tail = self.list_key(tail)?;
        let head = structural_ops::copy_into_list(self, head)?;
        let arena = self.list_arena_mut()?;
        let key = arena.prepend(head, tail).map_err(segmented_list_error)?;
        self.list_allocations = self.list_allocations.saturating_add(1);
        Ok(list_value(key))
    }

    pub(crate) fn list_first(
        &mut self,
        value: Value,
        representation: Option<lkjscript_core::StructuralRepresentationId>,
    ) -> Result<Value> {
        let key = self.list_key(value)?;
        let value = self
            .list_arena_mut()?
            .first_cloned(key)
            .map_err(segmented_list_error)?;
        structural_ops::copy_from_list(self, value, representation)
    }

    pub(crate) fn list_rest(&mut self, value: Value) -> Result<Value> {
        let key = self.list_key(value)?;
        self.list_arena_mut()?
            .rest(key)
            .map(list_value)
            .map_err(segmented_list_error)
    }

    pub(crate) fn list_is_empty(&self, value: Value) -> Result<bool> {
        let key = self.list_key(value)?;
        self.list_arena()?
            .is_empty(key)
            .map_err(segmented_list_error)
    }

    pub(crate) fn list_view(&self, value: Value) -> Result<Option<(Value, Value)>> {
        let key = self.list_key(value)?;
        self.list_arena()?
            .view(key)
            .map(|view| view.map(|(head, tail)| (*head, list_value(tail))))
            .map_err(segmented_list_error)
    }

    pub(crate) fn snapshot_list_aware_return(
        &self,
        value: Value,
    ) -> Result<lkjscript_core::OwnedValue> {
        let limit = usize::try_from(self.config.max_allocations).unwrap_or(usize::MAX);
        lkjscript_core::OwnedValue::from_segmented_list_snapshot(value, limit, |word| {
            let arena = self.list_arena()?;
            let key = arena.key_from_word(word).map_err(segmented_list_error)?;
            arena
                .collect_cloned(key, None)
                .map_err(segmented_list_error)
        })
        .and_then(|owned| {
            owned.retain_symbols(|index| match self.chunk.constant(index) {
                Some(lkjscript_core::Constant::Symbol(text)) => Ok(text.as_str()),
                _ => Err(Error::msg("invalid returned symbol constant index")),
            })
        })
    }

    pub(crate) fn list_reserved_bytes_estimate(&self) -> u64 {
        self.lists.as_ref().map_or(
            0,
            lkjscript_core::SegmentedListArena::reserved_bytes_estimate,
        )
    }

    fn list_key(&self, value: Value) -> Result<lkjscript_core::SegmentedListKey> {
        let arena = self.list_arena()?;
        if value.is_empty_list() {
            return Ok(arena.empty());
        }
        let word = value
            .as_segmented_list()
            .ok_or_else(|| Error::msg("list operation received a non-list value"))?;
        arena.key_from_word(word).map_err(segmented_list_error)
    }

    fn list_arena(&self) -> Result<&lkjscript_core::SegmentedListArena<Value>> {
        self.lists.as_ref().ok_or_else(|| {
            self.list_initialization_error.as_ref().map_or_else(
                || Error::msg("segmented-list arena is unavailable"),
                |error| Error::msg(error.to_string()),
            )
        })
    }

    fn list_arena_mut(&mut self) -> Result<&mut lkjscript_core::SegmentedListArena<Value>> {
        if let Some(error) = self.list_initialization_error.as_ref() {
            return Err(Error::msg(error.to_string()));
        }
        self.lists
            .as_mut()
            .ok_or_else(|| Error::msg("segmented-list arena is unavailable"))
    }
}

fn list_value(key: lkjscript_core::SegmentedListKey) -> Value {
    if key.is_empty() {
        Value::EMPTY_LIST
    } else {
        Value::from_segmented_list(key.to_word())
    }
}

fn segmented_list_error(error: lkjscript_core::SegmentedListError) -> Error {
    match error {
        lkjscript_core::SegmentedListError::Limit(limit) => Error::resource(
            ResourceLimitKind::Allocations,
            format!("segmented-list limit exceeded: {limit:?}"),
        ),
        _ => Error::msg(format!("segmented-list operation failed: {error:?}")),
    }
}
