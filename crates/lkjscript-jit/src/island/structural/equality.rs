use super::*;

const MAX_SEMANTIC_EQUALITY_NODES: usize = 65_536;

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
        if left_type != right_type {
            return Ok(false);
        }
        let left = self.export_comparison_copy(left, left_type)?;
        let right = self.export_comparison_copy(right, right_type)?;
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
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| NativeServiceError::ResourceLimitExceeded)?;
    pending.push((left, right));
    let mut visited = 0_usize;
    while let Some((left, right)) = pending.pop() {
        visited = visited
            .checked_add(1)
            .ok_or(NativeServiceError::ResourceLimitExceeded)?;
        if visited > MAX_SEMANTIC_EQUALITY_NODES {
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        if left.value_type != right.value_type {
            return Ok(false);
        }
        use SemanticPayload as Payload;
        let children = match (&left.payload, &right.payload) {
            (Payload::Inline(left), Payload::Inline(right)) if left == right => None,
            (Payload::Static(left), Payload::Static(right)) if left == right => None,
            (Payload::String(left), Payload::String(right)) if left == right => None,
            (Payload::Path(left), Payload::Path(right)) if left == right => None,
            (Payload::Bytes(left), Payload::Bytes(right)) if left == right => None,
            (Payload::ByteVector(left), Payload::ByteVector(right)) if left == right => None,
            (Payload::Product(left), Payload::Product(right)) => {
                Some((left.as_slice(), right.as_slice()))
            }
            (
                Payload::Enum {
                    tag: left_tag,
                    active_payload: left,
                },
                Payload::Enum {
                    tag: right_tag,
                    active_payload: right,
                },
            ) if left_tag == right_tag => Some((left.as_slice(), right.as_slice())),
            _ => return Ok(false),
        };
        let Some((left, right)) = children else {
            continue;
        };
        if left.len() != right.len() {
            return Ok(false);
        }
        if pending
            .len()
            .checked_add(left.len())
            .is_none_or(|nodes| nodes > MAX_SEMANTIC_EQUALITY_NODES)
        {
            return Err(NativeServiceError::ResourceLimitExceeded);
        }
        pending
            .try_reserve(left.len())
            .map_err(|_| NativeServiceError::ResourceLimitExceeded)?;
        pending.extend(left.iter().zip(right).rev());
    }
    Ok(true)
}

#[cfg(test)]
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
    fn semantic_equality_is_iterative_and_exact_for_deep_products() {
        let left = nested(2_048, 41);
        let equal = nested(2_048, 41);
        let different = nested(2_048, 42);
        assert_eq!(semantic_equal(&left, &equal), Ok(true));
        assert_eq!(semantic_equal(&left, &different), Ok(false));
    }
}
