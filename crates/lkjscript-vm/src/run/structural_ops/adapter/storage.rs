impl AggregateAdapters {
    pub(super) fn new(max_slots: u64) -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            max_slots: u32::try_from(max_slots).unwrap_or(u32::MAX),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.slots
            .iter()
            .all(|slot| !matches!(slot, AdapterSlot::Live { .. }))
    }

    fn allocate(&mut self, record: AdapterRecord) -> Result<Value> {
        self.free.try_reserve(1).map_err(|_| {
            Error::resource(
                ResourceLimitKind::HeapBytes,
                "aggregate adapter free-list capacity unavailable",
            )
        })?;
        let (slot, generation) = if let Some(slot) = self.free.pop() {
            let AdapterSlot::Vacant(generation) = self.slots[slot as usize] else {
                return Err(Error::msg("aggregate adapter free-list is corrupt"));
            };
            (slot, generation)
        } else {
            let slot = u32::try_from(self.slots.len()).map_err(|_| {
                Error::resource(
                    ResourceLimitKind::Allocations,
                    "aggregate adapter slot identity exhausted",
                )
            })?;
            if slot >= self.max_slots {
                return Err(Error::resource(
                    ResourceLimitKind::Allocations,
                    "aggregate adapter slot limit exceeded",
                ));
            }
            self.slots.try_reserve(1).map_err(|_| {
                Error::resource(
                    ResourceLimitKind::HeapBytes,
                    "aggregate adapter slot capacity unavailable",
                )
            })?;
            self.slots.push(AdapterSlot::Vacant(NonZeroU32::MIN));
            (slot, NonZeroU32::MIN)
        };
        self.slots[slot as usize] = AdapterSlot::Live { generation, record };
        Ok(Value::from_aggregate_adapter(adapter_word(
            slot, generation,
        )))
    }

    fn get(&self, value: Value) -> Result<AdapterRecord> {
        let (slot, generation) = adapter_parts(value)?;
        match self.slots.get(slot as usize) {
            Some(AdapterSlot::Live {
                generation: actual,
                record,
            }) if *actual == generation => Ok(*record),
            _ => Err(Error::msg("stale, forged, or consumed aggregate adapter")),
        }
    }

    fn take(&mut self, value: Value) -> Result<AdapterRecord> {
        let (slot, generation) = adapter_parts(value)?;
        let record = self.get(value)?;
        let replacement = if generation.get() == u32::MAX {
            AdapterSlot::Retired
        } else {
            self.free.push(slot);
            AdapterSlot::Vacant(
                NonZeroU32::new(generation.get() + 1)
                    .ok_or_else(|| Error::msg("aggregate adapter generation overflow"))?,
            )
        };
        self.slots[slot as usize] = replacement;
        Ok(record)
    }

    fn live_values(&self) -> Vec<Value> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(slot, value)| match value {
                AdapterSlot::Live { generation, .. } => u32::try_from(slot)
                    .ok()
                    .map(|slot| Value::from_aggregate_adapter(adapter_word(slot, *generation))),
                AdapterSlot::Vacant(_) | AdapterSlot::Retired => None,
            })
            .collect()
    }
}

const fn adapter_word(slot: u32, generation: NonZeroU32) -> u64 {
    ((generation.get() as u64) << 32) | slot as u64
}

fn adapter_parts(value: Value) -> Result<(u32, NonZeroU32)> {
    let word = value
        .as_aggregate_adapter()
        .ok_or_else(|| Error::msg("expected exact aggregate adapter"))?;
    let generation = NonZeroU32::new((word >> 32) as u32)
        .ok_or_else(|| Error::msg("aggregate adapter generation is invalid"))?;
    Ok((word as u32, generation))
}
