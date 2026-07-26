use super::*;

impl HeapCallDescriptor {
    pub fn new(
        operation: HeapOperation,
        input_types: Vec<ValueType>,
        result_type: ValueType,
        allocation: AllocationClass,
        store: StoreClass,
    ) -> Result<Self, PlanError> {
        if input_types.len() > 16 || input_types.len() != operation.expected_arity() {
            return Err(PlanError::InvalidHeapCall);
        }
        let descriptor = Self {
            operation,
            input_types,
            result_type,
            allocation,
            store,
        };
        if !descriptor.canonical_facts_are_valid() {
            return Err(PlanError::InvalidHeapCall);
        }
        Ok(descriptor)
    }

    #[must_use]
    pub fn operation(&self) -> &HeapOperation {
        &self.operation
    }

    #[must_use]
    pub fn input_types(&self) -> &[ValueType] {
        &self.input_types
    }

    #[must_use]
    pub const fn result_type(&self) -> ValueType {
        self.result_type
    }

    #[must_use]
    pub const fn allocation(&self) -> AllocationClass {
        self.allocation
    }

    #[must_use]
    pub const fn store(&self) -> StoreClass {
        self.store
    }

    pub(crate) fn canonical_facts_are_valid(&self) -> bool {
        let allocates = matches!(
            self.operation,
            HeapOperation::ConstantStr(_)
                | HeapOperation::EmptyStr
                | HeapOperation::ProductValue { .. }
                | HeapOperation::WithProductField { .. }
                | HeapOperation::EnumValue { .. }
                | HeapOperation::Cons
                | HeapOperation::BufNew
                | HeapOperation::BufClone
                | HeapOperation::BufFromStr
                | HeapOperation::BufToStr { .. }
                | HeapOperation::BufSlice { .. }
                | HeapOperation::StrAppend
                | HeapOperation::StrSlice
                | HeapOperation::StrFromByte
                | HeapOperation::StrFromI64
                | HeapOperation::StrFromF64
                | HeapOperation::F64FromI64Exact { .. }
                | HeapOperation::I64FromF64Exact { .. }
                | HeapOperation::I64FromF64Trunc { .. }
        );
        let expected_allocation = if allocates {
            AllocationClass::Bounded
        } else {
            AllocationClass::None
        };
        let expected_store = match self.operation {
            HeapOperation::BufSet | HeapOperation::BufSetU32 => StoreClass::Scalar,
            _ if allocates => StoreClass::Initialization,
            _ => StoreClass::None,
        };
        self.allocation == expected_allocation
            && self.store == expected_store
            && self.operation_types_are_valid()
    }
}
