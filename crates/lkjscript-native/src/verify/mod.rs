use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;

use crate::plan::{
    BlockId, FunctionDeclaration, FunctionId, FunctionPlan, LocalId, Operation, ReferenceType,
    RuntimeCallSlot, Signature, Terminator, TrapCode, ValueDefinition, ValueId, ValueType,
};
use crate::BackendLimits;

mod functions;
mod instructions;
mod layouts;
mod liveness;
mod plan;
mod roots;
mod terminators;

use functions::*;
use instructions::*;
use layouts::*;
use liveness::*;
pub(crate) use plan::verify_plan;
use roots::*;
use terminators::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationError {
    EmptyPlan,
    LimitExceeded(&'static str),
    MissingFunctionBody(FunctionId),
    DuplicateSourceFunction,
    UnsupportedSignature(FunctionId),
    MissingEntry(FunctionId),
    EmptyFunction(FunctionId),
    MissingTerminator(BlockId),
    InvalidTarget(BlockId),
    UnreachableBlock(BlockId),
    InvalidValue(ValueId),
    ValueNotAvailable(ValueId),
    InvalidLocal(LocalId),
    LocalNotInitialized(LocalId),
    TypeMismatch(&'static str),
    InvalidCall(FunctionId),
    InvalidReturn(FunctionId),
}

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPlan => formatter.write_str("machine plan has no functions"),
            Self::LimitExceeded(limit) => write!(formatter, "machine plan exceeds {limit} limit"),
            Self::MissingFunctionBody(function) => {
                write!(formatter, "function {function:?} has no body")
            }
            Self::DuplicateSourceFunction => {
                formatter.write_str("machine plan has duplicate source-function identities")
            }
            Self::UnsupportedSignature(function) => {
                write!(
                    formatter,
                    "function {function:?} has an unsupported native ABI signature"
                )
            }
            Self::MissingEntry(function) => {
                write!(formatter, "function {function:?} has no entry block")
            }
            Self::EmptyFunction(function) => {
                write!(formatter, "function {function:?} has no blocks")
            }
            Self::MissingTerminator(block) => {
                write!(formatter, "block {block:?} has no terminator")
            }
            Self::InvalidTarget(block) => {
                write!(formatter, "block {block:?} has an invalid branch target")
            }
            Self::UnreachableBlock(block) => write!(formatter, "block {block:?} is unreachable"),
            Self::InvalidValue(value) => write!(formatter, "value {value:?} is invalid"),
            Self::ValueNotAvailable(value) => {
                write!(formatter, "value {value:?} is not definitely available")
            }
            Self::InvalidLocal(local) => write!(formatter, "local {local:?} is invalid"),
            Self::LocalNotInitialized(local) => {
                write!(formatter, "local {local:?} is not definitely initialized")
            }
            Self::TypeMismatch(context) => write!(formatter, "type mismatch in {context}"),
            Self::InvalidCall(function) => write!(formatter, "invalid call to {function:?}"),
            Self::InvalidReturn(function) => write!(formatter, "invalid return in {function:?}"),
        }
    }
}

impl std::error::Error for VerificationError {}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CertifiedRoot {
    pub(crate) kind: crate::FrameHomeKind,
    pub(crate) reference_type: ReferenceType,
}

pub(crate) type FunctionRootRequirements = Vec<Option<Vec<CertifiedRoot>>>;

#[derive(Clone, Debug)]
pub struct VerifiedMachinePlan {
    pub(crate) functions: Vec<FunctionPlan>,
    pub(crate) static_bytes: Vec<Box<[u8]>>,
    pub(crate) root_requirements: Vec<FunctionRootRequirements>,
    pub(crate) limits: BackendLimits,
    pub(crate) work_units: u64,
}

impl VerifiedMachinePlan {
    #[must_use]
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    #[must_use]
    pub fn work_units(&self) -> u64 {
        self.work_units
    }
}
