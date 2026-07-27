use crate::{CapabilityKind, OperationIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationEffects(pub u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationOwnership {
    Observes,
    Allocates,
    Mutates,
    ConsumesResource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeLowering {
    Control,
    NumericConversion,
    Enum,
    RuntimeCall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSourceRelationship {
    BuiltinCall,
    ControlForm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationSemanticsRecord {
    pub identity: OperationIdentity,
    pub arity: u8,
    pub type_scheme: &'static str,
    pub generic_variables: &'static [&'static str],
    pub generic_constraints: &'static [&'static str],
    pub effects: OperationEffects,
    pub capability_requirements: &'static [CapabilityKind],
    pub ownership: OperationOwnership,
    pub may_trap: bool,
    pub may_diverge: bool,
    pub runtime_lowering: RuntimeLowering,
    pub semantic_source: SemanticSourceRelationship,
    pub legal_action_available: bool,
}

mod group_00;
mod group_01;
mod group_02;
mod group_03;
mod group_04;
mod group_05;
mod group_06;
mod group_07;
mod group_08;
mod group_09;
mod group_10;
mod group_11;
mod group_12;

pub(super) const fn required_operation_semantics(
    identity: OperationIdentity,
) -> &'static OperationSemanticsRecord {
    match operation_semantics_by_id(identity) {
        Some(record) => record,
        None => &group_00::RECORDS[0],
    }
}

pub const fn operation_semantics_by_id(
    identity: OperationIdentity,
) -> Option<&'static OperationSemanticsRecord> {
    let index = identity.as_u16() as usize;
    match index {
        0..=9 => Some(&group_00::RECORDS[index]),
        10..=19 => Some(&group_01::RECORDS[index - 10]),
        20..=29 => Some(&group_02::RECORDS[index - 20]),
        30..=39 => Some(&group_03::RECORDS[index - 30]),
        40..=49 => Some(&group_04::RECORDS[index - 40]),
        50..=59 => Some(&group_05::RECORDS[index - 50]),
        60..=69 => Some(&group_06::RECORDS[index - 60]),
        70..=79 => Some(&group_07::RECORDS[index - 70]),
        80..=89 => Some(&group_08::RECORDS[index - 80]),
        90..=99 => Some(&group_09::RECORDS[index - 90]),
        100..=109 => Some(&group_10::RECORDS[index - 100]),
        110..=119 => Some(&group_11::RECORDS[index - 110]),
        120..=122 => Some(&group_12::RECORDS[index - 120]),
        _ => None,
    }
}
