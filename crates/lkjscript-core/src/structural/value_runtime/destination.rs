use std::num::NonZeroU32;

use super::{
    SemanticValue, StructuralDestinationKey, StructuralEventKind, StructuralInitializationFailure,
    StructuralKind, StructuralType, StructuralValueError, StructuralValueLimit,
    StructuralValueRuntime, TreeFacts,
};

#[derive(Debug)]
pub(super) enum DestinationShape {
    Product,
    Enum(u16),
}

#[derive(Debug)]
pub(super) struct DestinationRecord {
    pub value_type: StructuralType,
    pub shape: DestinationShape,
    pub field_types: Vec<StructuralType>,
    pub values: Vec<Option<SemanticValue>>,
    pub facts: Vec<Option<TreeFacts>>,
    pub order: Vec<u16>,
    pub total: TreeFacts,
}

#[derive(Debug)]
pub(super) enum DestinationSlot {
    Vacant(NonZeroU32),
    Live {
        generation: NonZeroU32,
        record: DestinationRecord,
    },
    Retired,
}

impl StructuralValueRuntime {
    pub fn begin_product(
        &mut self,
        value_type: StructuralType,
        fields: Vec<StructuralType>,
    ) -> Result<StructuralDestinationKey, StructuralValueError> {
        if value_type.kind != StructuralKind::Product {
            return Err(StructuralValueError::WrongDestinationKind);
        }
        self.begin_destination(value_type, DestinationShape::Product, fields)
    }

    pub fn begin_enum(
        &mut self,
        value_type: StructuralType,
        tag: u16,
        active_payload: Vec<StructuralType>,
    ) -> Result<StructuralDestinationKey, StructuralValueError> {
        if value_type.kind != StructuralKind::Enum {
            return Err(StructuralValueError::WrongDestinationKind);
        }
        self.begin_destination(value_type, DestinationShape::Enum(tag), active_payload)
    }

    fn begin_destination(
        &mut self,
        value_type: StructuralType,
        shape: DestinationShape,
        field_types: Vec<StructuralType>,
    ) -> Result<StructuralDestinationKey, StructuralValueError> {
        if field_types.len() > usize::from(self.limits.max_fields) {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::Fields,
            ));
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(field_types.len())
            .map_err(|_| StructuralValueError::AllocationFailed)?;
        values.resize_with(field_types.len(), || None);
        let mut facts = Vec::new();
        facts
            .try_reserve_exact(field_types.len())
            .map_err(|_| StructuralValueError::AllocationFailed)?;
        facts.resize(field_types.len(), None);
        let mut order = Vec::new();
        order
            .try_reserve_exact(field_types.len())
            .map_err(|_| StructuralValueError::AllocationFailed)?;
        let key = self.allocate_destination(DestinationRecord {
            value_type,
            shape,
            field_types,
            values,
            facts,
            order,
            total: TreeFacts::default(),
        })?;
        self.metrics.destinations_created = self.metrics.destinations_created.saturating_add(1);
        self.metrics.live_destinations = self.metrics.live_destinations.saturating_add(1);
        self.record(StructuralEventKind::DestinationCreate, key.slot(), 0);
        Ok(key)
    }

    pub fn initialize_node(
        &mut self,
        key: StructuralDestinationKey,
        field: u16,
        value: SemanticValue,
    ) -> Result<(), StructuralInitializationFailure> {
        let result = self.initialize_node_inner(key, field, &value);
        let facts = match result {
            Ok(facts) => facts,
            Err(error) => return Err(StructuralInitializationFailure { error, value }),
        };
        let record = match self.destination_mut(key) {
            Ok(record) => record,
            Err(error) => return Err(StructuralInitializationFailure { error, value }),
        };
        let Some(total) = record.total.checked_add(facts) else {
            return Err(StructuralInitializationFailure {
                error: StructuralValueError::InvariantViolation,
                value,
            });
        };
        let index = usize::from(field);
        record.values[index] = Some(value);
        record.facts[index] = Some(facts);
        record.order.push(field);
        record.total = total;
        self.metrics.initializations = self.metrics.initializations.saturating_add(1);
        self.metrics.destination_fields_initialized = self
            .metrics
            .destination_fields_initialized
            .saturating_add(1);
        self.record(
            StructuralEventKind::Initialize,
            key.slot(),
            u64::from(field),
        );
        Ok(())
    }

    pub(super) fn initialize_node_inner(
        &self,
        key: StructuralDestinationKey,
        field: u16,
        value: &SemanticValue,
    ) -> Result<TreeFacts, StructuralValueError> {
        let record = self.destination(key)?;
        let index = usize::from(field);
        let expected = *record
            .field_types
            .get(index)
            .ok_or(StructuralValueError::FieldOutOfRange)?;
        if record.values[index].is_some() {
            return Err(StructuralValueError::FieldAlreadyInitialized);
        }
        self.require_type(value.value_type, expected)?;
        let facts = self.validate_tree(value)?;
        let total = record
            .total
            .checked_add(facts)
            .ok_or(StructuralValueError::ArithmeticOverflow)?;
        if total.nodes > self.limits.max_tree_nodes {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::TreeNodes,
            ));
        }
        if total.bytes > self.limits.max_payload_bytes {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::PayloadBytes,
            ));
        }
        Ok(facts)
    }

    pub(super) fn destination(
        &self,
        key: StructuralDestinationKey,
    ) -> Result<&DestinationRecord, StructuralValueError> {
        match self.destinations.get(key.slot() as usize) {
            Some(DestinationSlot::Live { generation, record })
                if generation.get() == key.generation() =>
            {
                Ok(record)
            }
            _ => Err(StructuralValueError::StaleDestination),
        }
    }

    pub(super) fn destination_mut(
        &mut self,
        key: StructuralDestinationKey,
    ) -> Result<&mut DestinationRecord, StructuralValueError> {
        match self.destinations.get_mut(key.slot() as usize) {
            Some(DestinationSlot::Live { generation, record })
                if generation.get() == key.generation() =>
            {
                Ok(record)
            }
            _ => Err(StructuralValueError::StaleDestination),
        }
    }
}
