use super::*;

pub(super) static NEXT_PLAN_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signature {
    pub(super) parameters: Vec<ValueType>,
    pub(super) result: ValueType,
}

impl Signature {
    pub fn new(parameters: Vec<ValueType>, result: ValueType) -> Result<Self, PlanError> {
        if parameters.len() > 16 {
            return Err(PlanError::TooManyParameters {
                count: parameters.len(),
                maximum: 16,
            });
        }
        Ok(Self { parameters, result })
    }

    #[must_use]
    pub fn parameters(&self) -> &[ValueType] {
        &self.parameters
    }

    #[must_use]
    pub const fn result(&self) -> ValueType {
        self.result
    }

    pub(crate) fn machine_parameter_count(&self) -> usize {
        self.parameters
            .iter()
            .filter(|parameter| **parameter != ValueType::Unit)
            .count()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceFunctionId(u64);

impl SourceFunctionId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SourceOrigin {
    Source(u64),
    Synthetic,
}

impl SourceOrigin {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self::Source(value)
    }

    #[must_use]
    pub const fn synthetic() -> Self {
        Self::Synthetic
    }

    #[must_use]
    pub const fn get(self) -> Option<u64> {
        match self {
            Self::Source(value) => Some(value),
            Self::Synthetic => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FunctionId {
    pub(crate) plan: u64,
    pub(crate) index: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlockId {
    pub(crate) function: FunctionId,
    pub(crate) index: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ValueId {
    pub(crate) function: FunctionId,
    pub(crate) index: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LocalId {
    pub(crate) function: FunctionId,
    pub(crate) index: u64,
}

macro_rules! host_index {
    ($name:ty) => {
        impl $name {
            #[must_use]
            pub fn host_index(self) -> Option<usize> {
                usize::try_from(self.index).ok()
            }
        }
    };
}

host_index!(FunctionId);
host_index!(BlockId);
host_index!(ValueId);
host_index!(LocalId);

#[cfg(test)]
mod tests {
    use super::{SourceFunctionId, SourceOrigin};

    #[test]
    fn source_identities_preserve_high_values_and_synthetic_is_disjoint() {
        let high = u64::from(u32::MAX) + 1;
        assert_eq!(SourceFunctionId::new(high).get(), high);
        assert_eq!(SourceOrigin::new(high).get(), Some(high));
        assert_eq!(SourceOrigin::synthetic().get(), None);
    }
}
