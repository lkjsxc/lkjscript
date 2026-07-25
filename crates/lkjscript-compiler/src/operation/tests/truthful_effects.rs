use super::*;

#[test]
fn removed_equality_names_and_effects_are_truthful() {
    assert!(Operation::from_name("eq").is_none());
    assert!(Operation::from_name("ne").is_none());
    assert_eq!(Operation::EqualValue.effects(), EffectSet::READS_MEMORY);
    assert_eq!(Operation::SameObject.effects(), EffectSet::READS_MEMORY);
    assert_eq!(Operation::F64BitsEqual.effects(), EffectSet::READS_MEMORY);
    assert_eq!(
        Operation::ListEqual.effects(),
        EffectSet::READS_MEMORY.union(EffectSet::MAY_TRAP)
    );
}
