use super::*;

impl JitStructuralRuntime {
    pub(in crate::island) fn retain_list_owner(
        &mut self,
        owner: NativeStructuralOwner,
    ) -> Result<Value, NativeServiceError> {
        let copy = self.copy_owner(owner)?;
        Ok(Value::from_structural_root(owner_key(copy)?))
    }

    pub(in crate::island) fn clone_list_owner(
        &mut self,
        value: Value,
        expected: lkjscript_native::ValueType,
    ) -> Result<NativeValue, NativeServiceError> {
        let key = value.as_structural_root().ok_or(NativeServiceError::Trap)?;
        let value_type = self
            .owners
            .get(&key.get())
            .copied()
            .ok_or(NativeServiceError::Trap)?;
        if expected != lkjscript_native::ValueType::StructuralOwner(value_type) {
            return Err(NativeServiceError::Trap);
        }
        self.copy_owner(NativeStructuralOwner::new(value_type, key.get()))
            .map(NativeValue::StructuralOwner)
    }

    pub(in crate::island) fn release_list_owner(
        &mut self,
        value: Value,
    ) -> Result<(), NativeServiceError> {
        let key = value.as_structural_root().ok_or(NativeServiceError::Trap)?;
        let value_type = self
            .owners
            .get(&key.get())
            .copied()
            .ok_or(NativeServiceError::Trap)?;
        self.drop_owner(NativeStructuralOwner::new(value_type, key.get()))
    }

    pub(in crate::island) fn list_owners_equal(
        &mut self,
        left: Value,
        right: Value,
    ) -> Result<bool, NativeServiceError> {
        let left = left.as_structural_root().ok_or(NativeServiceError::Trap)?;
        let right = right.as_structural_root().ok_or(NativeServiceError::Trap)?;
        self.semantic_owners_equal(left, right)
    }
}
