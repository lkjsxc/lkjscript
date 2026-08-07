use std::num::NonZeroU64;

use super::super::image::{discard_semantic, prepare_discard};
use super::{
    SemanticValue, StructuralDestinationKey, StructuralEventKind, StructuralImage,
    StructuralInitializationFailure, StructuralKind, StructuralType, StructuralValueError,
    StructuralValueRuntime, TreeFacts,
};

#[derive(Debug)]
pub(super) enum DestinationShape {
    Product,
    Enum(u64),
}

#[derive(Debug)]
pub(super) struct DestinationRecord {
    pub value_type: StructuralType,
    pub shape: DestinationShape,
    pub field_types: Vec<StructuralType>,
    pub values: Vec<Option<StructuralImage>>,
    pub facts: Vec<Option<TreeFacts>>,
    pub order: Vec<usize>,
    pub total: TreeFacts,
}

#[derive(Debug)]
pub(super) enum DestinationSlot {
    Vacant(NonZeroU64),
    Live {
        generation: NonZeroU64,
        key: StructuralDestinationKey,
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
        tag: u64,
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
        let next_allocation = self.next_allocation_event()?;
        let mut values = Vec::new();
        values.try_reserve_exact(field_types.len())?;
        values.resize_with(field_types.len(), || None);
        let mut facts = Vec::new();
        facts.try_reserve_exact(field_types.len())?;
        facts.resize(field_types.len(), None);
        let mut order = Vec::new();
        order.try_reserve_exact(field_types.len())?;
        let key = self.allocate_destination(DestinationRecord {
            value_type,
            shape,
            field_types,
            values,
            facts,
            order,
            total: TreeFacts::default(),
        })?;
        self.allocation_events = next_allocation;
        self.metrics.destinations_created = self.metrics.destinations_created.saturating_add(1);
        self.metrics.live_destinations = self.metrics.live_destinations.saturating_add(1);
        self.record(StructuralEventKind::DestinationCreate, key.get(), 0);
        Ok(key)
    }

    #[allow(clippy::result_large_err)]
    pub fn initialize_node(
        &mut self,
        key: StructuralDestinationKey,
        field: usize,
        value: SemanticValue,
    ) -> Result<(), StructuralInitializationFailure> {
        if let Err(error) = self.preflight_field_type(key, field, value.value_type) {
            return Err(StructuralInitializationFailure { error, value });
        }
        let facts = match self.validate_tree(&value) {
            Ok(facts) => facts,
            Err(error) => return Err(StructuralInitializationFailure { error, value }),
        };
        if let Err(error) = self.preflight_field(key, field, value.value_type, facts) {
            return Err(StructuralInitializationFailure { error, value });
        }
        let mut discard = match prepare_discard(facts) {
            Ok(stack) => stack,
            Err(error) => return Err(StructuralInitializationFailure { error, value }),
        };
        let image = match StructuralImage::build(&value, facts) {
            Ok(image) => image,
            Err(error) => return Err(StructuralInitializationFailure { error, value }),
        };
        match self.initialize_image(key, field, image, facts) {
            Ok(()) => {
                discard_semantic(value, &mut discard);
                Ok(())
            }
            Err(failure) => Err(StructuralInitializationFailure {
                error: failure.0,
                value,
            }),
        }
    }

    pub(super) fn preflight_field(
        &self,
        key: StructuralDestinationKey,
        field: usize,
        actual: StructuralType,
        facts: TreeFacts,
    ) -> Result<(), StructuralValueError> {
        self.preflight_field_type(key, field, actual)?;
        let record = self.destination(key)?;
        record
            .total
            .checked_add(facts)
            .ok_or(StructuralValueError::ArithmeticOverflow)?;
        Ok(())
    }

    pub(super) fn initialize_image(
        &mut self,
        key: StructuralDestinationKey,
        field: usize,
        image: StructuralImage,
        facts: TreeFacts,
    ) -> Result<(), Box<(StructuralValueError, StructuralImage)>> {
        if let Err(error) = self.preflight_field(key, field, image.root().value_type(), facts) {
            return Err(Box::new((error, image)));
        }
        let field_subject = match u64::try_from(field) {
            Ok(field) => field,
            Err(_) => {
                return Err(Box::new((StructuralValueError::ArithmeticOverflow, image)));
            }
        };
        let record = match self.destination_mut(key) {
            Ok(record) => record,
            Err(error) => return Err(Box::new((error, image))),
        };
        let Some(total) = record.total.checked_add(facts) else {
            return Err(Box::new((StructuralValueError::ArithmeticOverflow, image)));
        };
        let index = field;
        record.values[index] = Some(image);
        record.facts[index] = Some(facts);
        record.order.push(field);
        record.total = total;
        self.metrics.initializations = self.metrics.initializations.saturating_add(1);
        self.metrics.destination_fields_initialized = self
            .metrics
            .destination_fields_initialized
            .saturating_add(1);
        self.record(StructuralEventKind::Initialize, key.get(), field_subject);
        Ok(())
    }
}
