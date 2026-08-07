use super::destination::DestinationSlot;
use super::object_slab::{ObjectSlot, StructuralObject};
use super::{StructuralValueError, StructuralValueRuntime};

/// Invocation-policy observations for the currently retained structural store.
/// Counts are checked bookkeeping, never independent admission rules.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StructuralRuntimeAccounting {
    pub allocation_events: u64,
    pub retained_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StructuralExportAccounting {
    pub allocations: u64,
    pub retained_bytes: u64,
    pub output_bytes: u64,
}

impl StructuralValueRuntime {
    pub fn export_accounting(
        &mut self,
        key: super::StructuralValueKey,
        expected: super::StructuralType,
    ) -> Result<StructuralExportAccounting, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        let (image, facts) = match self.objects.get(root)? {
            StructuralObject::Owned { image, facts }
            | StructuralObject::Sealed { image, facts, .. } => (image, facts),
            StructuralObject::Static(_) => return Err(StructuralValueError::WrongOwnership),
        };
        Ok(StructuralExportAccounting {
            allocations: image.export_allocation_count()?,
            retained_bytes: image.retained_bytes_estimate()?,
            output_bytes: facts.bytes,
        })
    }

    pub fn accounting(&self) -> Result<StructuralRuntimeAccounting, StructuralValueError> {
        let allocation_events = self.allocation_events;
        let mut retained = 0_u64;
        add(&mut retained, self.runtime.retained_bytes_estimate()?)?;
        add(&mut retained, self.roots.retained_bytes_estimate()?)?;
        add_capacity::<ObjectSlot>(&mut retained, self.objects.slots.capacity())?;
        add_capacity::<u64>(&mut retained, self.objects.free.capacity())?;
        for slot in &self.objects.slots {
            let ObjectSlot::Live { object, .. } = slot else {
                continue;
            };
            match object {
                StructuralObject::Owned { image, .. } | StructuralObject::Sealed { image, .. } => {
                    add(&mut retained, image.retained_bytes_estimate()?)?;
                }
                StructuralObject::Static(_) => {}
            }
        }
        add_capacity::<DestinationSlot>(&mut retained, self.destinations.capacity())?;
        add_capacity::<u64>(&mut retained, self.free_destinations.capacity())?;
        for slot in &self.destinations {
            let DestinationSlot::Live { record, .. } = slot else {
                continue;
            };
            add_capacity::<super::StructuralType>(&mut retained, record.field_types.capacity())?;
            add_capacity::<Option<super::StructuralImage>>(
                &mut retained,
                record.values.capacity(),
            )?;
            add_capacity::<Option<super::TreeFacts>>(&mut retained, record.facts.capacity())?;
            add_capacity::<usize>(&mut retained, record.order.capacity())?;
            for image in record.values.iter().flatten() {
                add(&mut retained, image.retained_bytes_estimate()?)?;
            }
        }
        add_capacity::<super::ViewSlot>(&mut retained, self.views.capacity())?;
        add_capacity::<u64>(&mut retained, self.free_views.capacity())?;
        add_capacity::<(u64, super::PrivateTokenRecord)>(
            &mut retained,
            self.private_tokens.capacity(),
        )?;
        add(&mut retained, self.events.retained_bytes_estimate()?)?;
        add_capacity::<super::DestinationCleanupReport>(
            &mut retained,
            self.cleanup_reports.capacity(),
        )?;
        for report in &self.cleanup_reports {
            add_capacity::<usize>(&mut retained, report.cleanup_order.capacity())?;
        }
        Ok(StructuralRuntimeAccounting {
            allocation_events,
            retained_bytes: retained,
        })
    }
}

fn add_capacity<T>(total: &mut u64, capacity: usize) -> Result<(), StructuralValueError> {
    let bytes = u64::try_from(capacity)
        .ok()
        .and_then(|value| value.checked_mul(std::mem::size_of::<T>() as u64))
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    add(total, bytes)
}

fn add(total: &mut u64, amount: u64) -> Result<(), StructuralValueError> {
    *total = total
        .checked_add(amount)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    Ok(())
}
