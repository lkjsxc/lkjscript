impl AggregateAdapters {
    pub(super) fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            tokens: HashMap::new(),
            next_token: NonZeroU64::new(1),
            allocations: 0,
        }
    }

    pub(super) fn accounting(&self) -> Result<(u64, u64)> {
        let slots = u64::try_from(self.slots.capacity())
            .ok()
            .and_then(|capacity| capacity.checked_mul(std::mem::size_of::<AdapterSlot>() as u64))
            .ok_or_else(|| Error::host("aggregate adapter byte accounting overflow"))?;
        let free = u64::try_from(self.free.capacity())
            .ok()
            .and_then(|capacity| capacity.checked_mul(std::mem::size_of::<u64>() as u64))
            .ok_or_else(|| Error::host("aggregate adapter byte accounting overflow"))?;
        let tokens = u64::try_from(self.tokens.capacity())
            .ok()
            .and_then(|capacity| {
                capacity.checked_mul(std::mem::size_of::<(u64, AdapterIdentity)>() as u64)
            })
            .ok_or_else(|| Error::host("aggregate adapter byte accounting overflow"))?;
        Ok((
            self.allocations,
            slots
                .checked_add(free)
                .and_then(|bytes| bytes.checked_add(tokens))
                .ok_or_else(|| Error::host("aggregate adapter byte accounting overflow"))?,
        ))
    }

    pub(super) fn is_empty(&self) -> bool {
        self.slots
            .iter()
            .all(|slot| !matches!(slot, AdapterSlot::Live { .. }))
    }

    fn allocate(&mut self, record: AdapterRecord) -> Result<Value> {
        let next_allocations = self
            .allocations
            .checked_add(1)
            .ok_or_else(|| Error::host("aggregate adapter allocation accounting overflow"))?;
        let (slot, generation, reused) = if let Some(&slot) = self.free.last() {
            let index = usize::try_from(slot)
                .map_err(|_| Error::host("aggregate adapter slot exceeds platform"))?;
            let Some(AdapterSlot::Vacant(generation)) = self.slots.get(index) else {
                return Err(Error::msg("aggregate adapter free-list is corrupt"));
            };
            (slot, *generation, true)
        } else {
            self.slots.try_reserve(1).map_err(|_| {
                Error::resource(
                    ResourceLimitKind::HeapBytes,
                    "aggregate adapter slot capacity unavailable",
                )
            })?;
            let slot = u64::try_from(self.slots.len())
                .map_err(|_| Error::host("aggregate adapter slot identity exceeds u64"))?;
            (slot, NonZeroU64::MIN, false)
        };
        self.tokens.try_reserve(1).map_err(|_| {
            Error::resource(
                ResourceLimitKind::HeapBytes,
                "aggregate adapter token capacity unavailable",
            )
        })?;
        let token = self
            .next_token
            .ok_or_else(|| Error::host("aggregate adapter token identity exhausted"))?;
        self.next_token = token.get().checked_add(1).and_then(NonZeroU64::new);
        let identity = AdapterIdentity { slot, generation };
        if self.tokens.contains_key(&token.get()) {
            return Err(Error::host("aggregate adapter token collision"));
        }
        let index = usize::try_from(slot)
            .map_err(|_| Error::host("aggregate adapter slot exceeds platform"))?;
        if reused {
            let _ = self.free.pop();
            self.slots[index] = AdapterSlot::Live {
                generation,
                token,
                record,
            };
        } else {
            self.slots.push(AdapterSlot::Live {
                generation,
                token,
                record,
            });
        }
        self.tokens.insert(token.get(), identity);
        self.allocations = next_allocations;
        Ok(Value::from_aggregate_adapter(token.get()))
    }

    fn get(&self, value: Value) -> Result<AdapterRecord> {
        let (token, identity) = self.adapter_identity(value)?;
        let index = usize::try_from(identity.slot)
            .map_err(|_| Error::host("aggregate adapter slot exceeds platform"))?;
        match self.slots.get(index) {
            Some(AdapterSlot::Live {
                generation,
                token: current,
                record,
            }) if *current == token && *generation == identity.generation => Ok(*record),
            _ => Err(Error::msg("stale, forged, or consumed aggregate adapter")),
        }
    }

    fn take(&mut self, value: Value) -> Result<AdapterRecord> {
        let (token, identity) = self.adapter_identity(value)?;
        let record = self.get(value)?;
        let index = usize::try_from(identity.slot)
            .map_err(|_| Error::host("aggregate adapter slot exceeds platform"))?;
        let replacement = if identity.generation.get() == u64::MAX {
            AdapterSlot::Retired
        } else {
            self.free.try_reserve(1).map_err(|_| {
                Error::resource(
                    ResourceLimitKind::HeapBytes,
                    "aggregate adapter free-list capacity unavailable",
                )
            })?;
            AdapterSlot::Vacant(
                identity
                    .generation
                    .get()
                    .checked_add(1)
                    .and_then(NonZeroU64::new)
                    .ok_or_else(|| Error::host("aggregate adapter generation overflow"))?,
            )
        };
        let AdapterSlot::Live { token: current, .. } = self.slots[index] else {
            return Err(Error::msg("stale, forged, or consumed aggregate adapter"));
        };
        if current != token {
            return Err(Error::msg("stale, forged, or consumed aggregate adapter"));
        }
        self.slots[index] = replacement;
        if !matches!(&self.slots[index], AdapterSlot::Retired) {
            self.free.push(identity.slot);
        }
        self.tokens.remove(&token.get());
        Ok(record)
    }

    fn live_values(&self) -> Vec<Value> {
        self.slots
            .iter()
            .filter_map(|value| match value {
                AdapterSlot::Live { token, .. } => Some(Value::from_aggregate_adapter(token.get())),
                AdapterSlot::Vacant(_) | AdapterSlot::Retired => None,
            })
            .collect()
    }

    fn adapter_identity(&self, value: Value) -> Result<(NonZeroU64, AdapterIdentity)> {
        let word = value
            .as_aggregate_adapter()
            .ok_or_else(|| Error::msg("expected exact aggregate adapter"))?;
        let token = NonZeroU64::new(word)
            .ok_or_else(|| Error::msg("aggregate adapter token is zero"))?;
        let identity = self
            .tokens
            .get(&word)
            .copied()
            .ok_or_else(|| Error::msg("stale, forged, or consumed aggregate adapter"))?;
        Ok((token, identity))
    }
}
