use super::super::*;
use super::fixtures::*;
use super::witnesses::witness;
use crate::hir;
use lkjscript_core::Result;

#[test]
fn nested_copy_lists_select_exact_witnesses() -> Result<()> {
    let first = hir::Type::List(Box::new(hir::Type::I64));
    let second = hir::Type::List(Box::new(first.clone()));
    let third = hir::Type::List(Box::new(second.clone()));
    let plan = derive(&program(
        third.clone(),
        fake(third.clone()),
        Vec::new(),
        Vec::new(),
    ))?;
    for (ty, element) in [(second.clone(), first), (third, second)] {
        let memory_ty = producer::memory_type(&ty);
        let ty_fact = fact(&plan, &memory_ty)?;
        assert_eq!(ty_fact.mode, MemoryAggregateMode::ImmutableValue);
        assert_eq!(ty_fact.closure.class, MemoryClosureClass::RegionClosed);
        assert!(!ty_fact.contains_borrow);
        assert!(!ty_fact.contains_dynamic_owner);
        assert_eq!(ty_fact.copy_share, MemoryCopySharePlan::RegionHandleCopy);

        let ty_witness = witness(&plan, &memory_ty)?;
        assert_eq!(
            ty_witness.facts.requirement,
            MemoryWitnessRequirement::Concrete
        );
        assert_eq!(ty_witness.facts.domain, MemoryDomain::OrdinaryRegion);
        assert_eq!(
            ty_witness.facts.process_codec,
            MemoryProcessCodecEligibility::Eligible
        );
        let list = ty_witness.facts.list.as_ref().ok_or_else(|| {
            lkjscript_core::Error::msg("nested segmented list witness is missing")
        })?;
        assert_eq!(
            list.element,
            witness(&plan, &producer::memory_type(&element))?.id
        );
        assert!(list.selected);
        assert_eq!(list.eligibility, MemoryListElementEligibility::Copy);
        assert_eq!(list.storage, MemoryListStorageKind::SegmentedSessionRegion);
        assert_eq!(list.segment_capacity, 32);
    }
    Ok(())
}

#[test]
fn nested_structural_owner_lists_select_exact_witnesses() -> Result<()> {
    let inner = hir::Type::List(Box::new(hir::Type::Str));
    let outer = hir::Type::List(Box::new(inner.clone()));
    let plan = derive(&program(
        outer.clone(),
        fake(outer.clone()),
        Vec::new(),
        Vec::new(),
    ))?;
    for ty in [inner, outer] {
        let fact = fact(&plan, &producer::memory_type(&ty))?;
        assert_eq!(fact.closure.class, MemoryClosureClass::RegionClosed);
        assert_eq!(fact.copy_share, MemoryCopySharePlan::RegionHandleCopy);
        let list = witness(&plan, &producer::memory_type(&ty))?
            .facts
            .list
            .as_ref()
            .ok_or_else(|| {
                lkjscript_core::Error::msg("selected structural-owner list witness is missing")
            })?;
        assert!(list.selected);
    }
    Ok(())
}

#[test]
fn nested_affine_or_unresolved_lists_remain_rejected() -> Result<()> {
    for leaf in [hir::Type::Param("unknown".into()), hir::Type::Bytes] {
        let inner = hir::Type::List(Box::new(leaf));
        let outer = hir::Type::List(Box::new(inner));
        let plan = derive(&program(
            outer.clone(),
            fake(outer.clone()),
            Vec::new(),
            Vec::new(),
        ))?;
        let ty_fact = fact(&plan, &producer::memory_type(&outer))?;
        assert_eq!(ty_fact.closure.class, MemoryClosureClass::Unresolved);
        assert_eq!(
            ty_fact.closure.blocker_reason,
            Some(MemoryBlockerReason::ListElementWitnessRequired)
        );
        assert_eq!(ty_fact.copy_share, MemoryCopySharePlan::Unsupported);

        let ty_witness = witness(&plan, &producer::memory_type(&outer))?;
        assert_eq!(ty_witness.facts.domain, MemoryDomain::UnsupportedRuntime);
        assert_eq!(
            ty_witness.facts.process_codec,
            MemoryProcessCodecEligibility::Ineligible
        );
        assert!(
            !ty_witness
                .facts
                .list
                .as_ref()
                .ok_or_else(|| {
                    lkjscript_core::Error::msg("rejected nested list witness is missing")
                })?
                .selected
        );
    }
    Ok(())
}
