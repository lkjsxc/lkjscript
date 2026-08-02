impl VerifiedTypes<'_> {
    fn verified_selected_list_element(
        &self,
        ty: &Type,
        fact: &VerifiedExpectedType,
    ) -> bool {
        if fact.derived.contains_borrow
            || fact.derived.closure.class == MemoryClosureClass::Unresolved
        {
            return false;
        }
        if matches!(ty, Type::List(_)) {
            return fact.derived.mode == MemoryAggregateMode::ImmutableValue
                && fact.derived.closure.class == MemoryClosureClass::RegionClosed
                && !fact.derived.contains_dynamic_owner
                && super::witness::verified_witness_list_element(ty, &fact.derived)
                    == MemoryListElementEligibility::Copy;
        }
        fact.derived.closure.class == MemoryClosureClass::Deterministic
            && matches!(
                super::witness::verified_witness_list_element(ty, &fact.derived),
                MemoryListElementEligibility::Copy | MemoryListElementEligibility::ImmutableValue
            )
            && matches!(
                ty,
                Type::Str | Type::Path | Type::Product(_) | Type::Enum { .. }
            )
    }
}
