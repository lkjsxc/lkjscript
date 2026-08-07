#[derive(Clone, Copy, Debug)]
struct OwnerRecord {
    representation: StructuralRepresentationId,
    value_type: StructuralType,
    taken_from: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
enum ListOwnerRecord {
    Typed(OwnerRecord),
    Host(StructuralType),
}

impl ListOwnerRecord {
    const fn value_type(self) -> StructuralType {
        match self {
            Self::Typed(record) => record.value_type,
            Self::Host(value_type) => value_type,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ViewRecord {
    representation: StructuralRepresentationId,
    utf8: bool,
}

#[derive(Clone, Copy, Debug)]
struct DestinationRecord {
    destination: StructuralDestinationId,
    value_type: StructuralType,
    taken_from: Option<usize>,
}

pub(crate) struct StructuralInvocation {
    pub(super) runtime: StructuralValueRuntime,
    owners: BTreeMap<u64, OwnerRecord>,
    list_owners: BTreeMap<u64, ListOwnerRecord>,
    host_owners: BTreeMap<u64, StructuralType>,
    views: BTreeMap<u64, ViewRecord>,
    destinations: BTreeMap<u64, DestinationRecord>,
    adapters: adapter::AggregateAdapters,
}

impl StructuralInvocation {
    pub(super) fn new(max_adapters: Option<u64>) -> Result<Self> {
        let runtime = StructuralValueRuntime::new(StructuralValueRuntimeLimits::default())
            .map_err(map_value_error)?;
        Ok(Self {
            runtime,
            owners: BTreeMap::new(),
            list_owners: BTreeMap::new(),
            host_owners: BTreeMap::new(),
            views: BTreeMap::new(),
            destinations: BTreeMap::new(),
            adapters: adapter::AggregateAdapters::new(max_adapters),
        })
    }

    fn owner(&self, value: Value) -> Result<(StructuralValueKey, OwnerRecord)> {
        let key = value
            .as_structural_root()
            .ok_or_else(|| Error::msg("expected exact structural owner value"))?;
        let record = self
            .owners
            .get(&key.get())
            .copied()
            .ok_or_else(|| Error::msg("stale, forged, or unregistered structural owner"))?;
        Ok((key, record))
    }

    fn view(&self, value: Value) -> Result<(StructuralViewKey, ViewRecord)> {
        let word = value
            .as_structural_view()
            .ok_or_else(|| Error::msg("expected exact structural view value"))?;
        let key = StructuralViewKey::from_word(word)
            .ok_or_else(|| Error::msg("invalid structural view representation"))?;
        let record = self
            .views
            .get(&word)
            .copied()
            .ok_or_else(|| Error::msg("stale, forged, or unregistered structural view"))?;
        Ok((key, record))
    }

    fn destination(&self, value: Value) -> Result<(StructuralDestinationKey, DestinationRecord)> {
        let word = value
            .as_structural_destination()
            .ok_or_else(|| Error::msg("expected exact structural destination value"))?;
        let key = StructuralDestinationKey::from_word(word)
            .ok_or_else(|| Error::msg("invalid structural destination representation"))?;
        let record =
            self.destinations.get(&word).copied().ok_or_else(|| {
                Error::msg("stale, forged, or unregistered structural destination")
            })?;
        Ok((key, record))
    }

    fn register_owner(
        &mut self,
        key: StructuralValueKey,
        representation: StructuralRepresentationId,
        value_type: StructuralType,
    ) -> Result<Value> {
        if self
            .owners
            .insert(
                key.get(),
                OwnerRecord {
                    representation,
                    value_type,
                    taken_from: None,
                },
            )
            .is_some()
        {
            return Err(Error::msg("duplicate structural owner registry key"));
        }
        Ok(Value::from_structural_root(key))
    }

    fn register_host_owner(
        &mut self,
        key: StructuralValueKey,
        value_type: StructuralType,
    ) -> Result<Value> {
        if self.host_owners.insert(key.get(), value_type).is_some() {
            return Err(Error::msg("duplicate host structural owner registry key"));
        }
        Ok(Value::from_structural_root(key))
    }

    fn register_view(
        &mut self,
        key: StructuralViewKey,
        representation: StructuralRepresentationId,
        _value_type: StructuralType,
        utf8: bool,
    ) -> Result<Value> {
        if self
            .views
            .insert(
                key.get(),
                ViewRecord {
                    representation,
                    utf8,
                },
            )
            .is_some()
        {
            return Err(Error::msg("duplicate structural view registry key"));
        }
        Ok(Value::from_structural_view(key))
    }

    fn register_destination(
        &mut self,
        key: StructuralDestinationKey,
        destination: StructuralDestinationId,
        value_type: StructuralType,
    ) -> Result<Value> {
        if self
            .destinations
            .insert(
                key.get(),
                DestinationRecord {
                    destination,
                    value_type,
                    taken_from: None,
                },
            )
            .is_some()
        {
            return Err(Error::msg("duplicate structural destination registry key"));
        }
        Ok(Value::from_structural_destination(key))
    }

    fn is_empty(&self) -> bool {
        self.owners.is_empty()
            && self.list_owners.is_empty()
            && self.host_owners.is_empty()
            && self.views.is_empty()
            && self.destinations.is_empty()
            && self.adapters.is_empty()
    }
}

include!("invocation/list_owners.rs");
