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
pub struct SourceFunctionId(u32);

impl SourceFunctionId {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceOrigin(u32);

impl SourceOrigin {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FunctionId {
    pub(crate) plan: u64,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlockId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ValueId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LocalId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}
