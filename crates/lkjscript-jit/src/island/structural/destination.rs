use super::*;

impl JitStructuralRuntime {
    pub(super) fn create_destination(
        &mut self,
        aggregate: &StructuralAggregateDescriptor,
    ) -> Result<NativeStructuralDestination, NativeServiceError> {
        self.reserve_construction()?;
        self.note_call();
        let value_type = core_type(aggregate.value_type())?;
        let fields = aggregate
            .fields()
            .iter()
            .copied()
            .map(core_type)
            .collect::<Result<Vec<_>, _>>()?;
        let key = match aggregate.kind() {
            StructuralAggregateKind::Product => self.runtime.begin_product(value_type, fields),
            StructuralAggregateKind::Enum(tag) => self.runtime.begin_enum(value_type, tag, fields),
        }
        .map_err(|error| self.map_error(error))?;
        Ok(NativeStructuralDestination::new(
            aggregate.destination(0),
            key.get(),
        ))
    }

    pub(super) fn initialize_destination(
        &mut self,
        destination: NativeStructuralDestination,
        value: NativeValue,
        aggregate: &StructuralAggregateDescriptor,
        field: u16,
    ) -> Result<NativeStructuralDestination, NativeServiceError> {
        self.note_call();
        if destination.destination_type() != aggregate.destination(field) {
            return Err(NativeServiceError::Trap);
        }
        let expected = aggregate.fields()[usize::from(field)];
        let mut consumed_owner = None;
        let value = match value {
            NativeValue::StructuralOwner(owner) if owner.structural_type() == expected => {
                let key = owner_key(owner)?;
                consumed_owner = Some(key.get());
                Value::from_structural_root(key)
            }
            NativeValue::Unit if expected.kind() == lkjscript_native::StructuralKind::Unit => {
                Value::UNIT
            }
            NativeValue::Bool(value)
                if expected.kind() == lkjscript_native::StructuralKind::Bool =>
            {
                Value::from_bool(value)
            }
            NativeValue::I64(value) if expected.kind() == lkjscript_native::StructuralKind::I64 => {
                Value::from_i64(value)
            }
            NativeValue::F64Bits(bits)
                if expected.kind() == lkjscript_native::StructuralKind::F64 =>
            {
                Value::from_f64_bits(bits)
            }
            _ => return Err(NativeServiceError::Trap),
        };
        self.runtime
            .initialize_value(destination_key(destination)?, field, value)
            .map_err(|error| self.map_error(error))?;
        if let Some(key) = consumed_owner {
            self.owners.remove(&key);
        }
        let next = field
            .checked_add(1)
            .ok_or(NativeServiceError::HostFailure)?;
        Ok(NativeStructuralDestination::new(
            aggregate.destination(next),
            destination.opaque_word(),
        ))
    }

    pub(super) fn finish_destination(
        &mut self,
        destination: NativeStructuralDestination,
        aggregate: &StructuralAggregateDescriptor,
    ) -> Result<NativeStructuralOwner, NativeServiceError> {
        self.note_call();
        let initialized =
            u16::try_from(aggregate.fields().len()).map_err(|_| NativeServiceError::HostFailure)?;
        if destination.destination_type() != aggregate.destination(initialized) {
            return Err(NativeServiceError::Trap);
        }
        let key = self
            .runtime
            .finish_destination(destination_key(destination)?)
            .map_err(|error| self.map_error(error))?;
        self.owners.insert(key.get(), aggregate.value_type());
        Ok(NativeStructuralOwner::new(
            aggregate.value_type(),
            key.get(),
        ))
    }

    pub(super) fn abort_destination(
        &mut self,
        destination: NativeStructuralDestination,
    ) -> Result<(), NativeServiceError> {
        self.note_call();
        self.runtime
            .abort_destination(destination_key(destination)?)
            .map(|_| ())
            .map_err(|error| self.map_error(error))
    }
}
