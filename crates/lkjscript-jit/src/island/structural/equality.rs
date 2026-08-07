use super::*;

impl JitStructuralRuntime {
    pub(super) fn semantic_owners_equal(
        &mut self,
        left: StructuralValueKey,
        right: StructuralValueKey,
    ) -> Result<bool, NativeServiceError> {
        let left_type = self
            .owners
            .get(&left.get())
            .copied()
            .ok_or(NativeServiceError::Trap)?;
        let right_type = self
            .owners
            .get(&right.get())
            .copied()
            .ok_or(NativeServiceError::Trap)?;
        if left_type.value_type != right_type.value_type {
            return Ok(false);
        }
        let left = self.export_comparison_copy(left, left_type.value_type)?;
        let right = self.export_comparison_copy(right, right_type.value_type)?;
        semantic_equal(&left, &right)
    }

    fn export_comparison_copy(
        &mut self,
        key: StructuralValueKey,
        value_type: StructuralTypeIdentity,
    ) -> Result<SemanticValue, NativeServiceError> {
        let copy = self.copy_owner(NativeStructuralOwner::new(value_type, key.get()))?;
        match self.export(copy) {
            Ok(value) => Ok(value),
            Err(error) => {
                let _ = self.drop_owner(copy);
                Err(error)
            }
        }
    }
}

fn semantic_equal(left: &SemanticValue, right: &SemanticValue) -> Result<bool, NativeServiceError> {
    left.try_equal(right)
        .map_err(|_| NativeServiceError::ResourceLimitExceeded)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::num::NonZeroU64;

    fn ty(kind: StructuralKind) -> StructuralType {
        StructuralType::new(
            LayoutIdentity::new(NonZeroU64::MIN),
            SemanticTypeIdentity::new(NonZeroU64::MIN),
            kind,
        )
    }

    fn nested(depth: usize, leaf: i64) -> SemanticValue {
        let mut value = SemanticValue::new(
            ty(StructuralKind::I64),
            SemanticPayload::Inline(InlineStructuralValue::I64(leaf)),
        );
        for _ in 0..depth {
            let mut fields = lkjscript_core::SemanticChildren::new();
            fields.push(value);
            value = SemanticValue::new(
                ty(StructuralKind::Product),
                SemanticPayload::Product(fields),
            );
        }
        value
    }

    #[test]
    fn semantic_equality_crosses_former_native_node_limit() {
        fn wide(fields: usize) -> SemanticValue {
            let mut children = lkjscript_core::SemanticChildren::new();
            for value in 0..fields {
                children.push(SemanticValue::new(
                    ty(StructuralKind::I64),
                    SemanticPayload::Inline(InlineStructuralValue::I64(
                        i64::try_from(value).expect("test field fits i64"),
                    )),
                ));
            }
            SemanticValue::new(
                ty(StructuralKind::Product),
                SemanticPayload::Product(children),
            )
        }

        let left = wide(65_537);
        let equal = wide(65_537);
        assert_eq!(semantic_equal(&left, &equal), Ok(true));
    }

    #[test]
    fn semantic_equality_is_iterative_and_exact_for_deep_products() {
        let left = nested(2_048, 41);
        let equal = nested(2_048, 41);
        let different = nested(2_048, 42);
        assert_eq!(semantic_equal(&left, &equal), Ok(true));
        assert_eq!(semantic_equal(&left, &different), Ok(false));
    }
}
