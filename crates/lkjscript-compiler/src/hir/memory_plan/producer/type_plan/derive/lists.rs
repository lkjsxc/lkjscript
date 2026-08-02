impl TypePlanner<'_> {
    fn selected_list_element(&self, ty: &Type, fact: &MemoryTypeFact) -> bool {
        if fact.contains_borrow || fact.closure.class == MemoryClosureClass::Unresolved {
            return false;
        }
        if matches!(ty, Type::List(_)) {
            return fact.mode == MemoryAggregateMode::ImmutableValue
                && fact.closure.class == MemoryClosureClass::RegionClosed
                && !fact.contains_dynamic_owner
                && self
                    .witnesses
                    .iter()
                    .find(|item| item.id == fact.witness)
                    .filter(|item| item.facts.requirement == MemoryWitnessRequirement::Concrete)
                    .and_then(|item| item.facts.list.as_ref())
                    .is_some_and(|list| {
                        list.selected
                            && matches!(
                                list.eligibility,
                                MemoryListElementEligibility::Copy
                                    | MemoryListElementEligibility::ImmutableValue
                            )
                            && list.storage == MemoryListStorageKind::SegmentedSessionRegion
                            && list.segment_capacity == 32
                    });
        }
        fact.closure.class == MemoryClosureClass::Deterministic
            && matches!(
                witness_list_element(
                    ty,
                    &DerivedType {
                        mode: fact.mode,
                        closure: fact.closure.clone(),
                        contains_borrow: fact.contains_borrow,
                        contains_dynamic_owner: fact.contains_dynamic_owner,
                    }
                ),
                MemoryListElementEligibility::Copy
                    | MemoryListElementEligibility::ImmutableValue
            )
            && matches!(
                ty,
                Type::Str | Type::Path | Type::Product(_) | Type::Enum { .. }
            )
    }
}
