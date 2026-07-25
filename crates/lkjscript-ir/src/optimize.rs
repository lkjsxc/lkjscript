use std::collections::BTreeSet;
use std::fmt;

use crate::{
    canonical_block_order, copy_propagate, direct_call_resolution, effect_aware_dce,
    empty_block_forwarding, simplify_branches, unreachable_blocks, verify, BlockId, Constant,
    EffectSet, FailureBehavior, Function, FunctionId, Instruction, InstructionKind, Program,
    RuntimeOp, Safepoint, SsaType, ValueId, VerifiedProgram,
};

/// Deterministic resource bounds for the proof-producing optimization slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptimizationLimits {
    pub max_work_units: u64,
    pub max_certificate_records: u64,
    pub max_certificate_bytes: u64,
    pub max_instruction_growth: u64,
    pub max_iterations: u64,
}

impl Default for OptimizationLimits {
    fn default() -> Self {
        Self {
            max_work_units: 16 * 1024 * 1024,
            max_certificate_records: 65_536,
            max_certificate_bytes: 4 * 1024 * 1024,
            max_instruction_growth: 0,
            max_iterations: 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimizationFailureCode {
    InputVerification,
    BudgetExceeded,
    CertificateMismatch,
    IllegalEdit,
    CandidateMismatch,
    OutputVerification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizationError {
    code: OptimizationFailureCode,
    detail: String,
}

impl OptimizationError {
    fn new(code: OptimizationFailureCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub const fn code(&self) -> OptimizationFailureCode {
        self.code
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for OptimizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "optimization {:?}: {}", self.code, self.detail)
    }
}

impl std::error::Error for OptimizationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimizationEditKind {
    AlgebraicIdentity,
    GlobalValueNumbering,
    CheckedI64GlobalValueNumbering,
}

/// One ordered edit over the stable IDs of the verified baseline input.
///
/// Operation and operands are repeated deliberately: the certificate verifier
/// checks them against the input rather than trusting edit discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizationCertificateRecord {
    pub sequence: u64,
    pub function: FunctionId,
    pub block: BlockId,
    pub value: ValueId,
    pub kind: OptimizationEditKind,
    pub expected_operation: RuntimeOp,
    pub expected_operands: Vec<ValueId>,
    pub replacement: ValueId,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OptimizationCertificate {
    pub records: Vec<OptimizationCertificateRecord>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OptimizationStats {
    pub input_instructions: u64,
    pub output_instructions: u64,
    pub work_units: u64,
    pub certificate_records: u64,
    pub certificate_bytes: u64,
    pub instruction_growth: u64,
    pub iterations: u64,
    pub algebraic_rewrites: u64,
    pub gvn_rewrites: u64,
    pub checked_i64_rewrites: u64,
    pub cleanup_removed_instructions: u64,
}

/// Opaque authority required by optimizing lowering.
///
/// There is intentionally no constructor from `Program` or `VerifiedProgram`.
#[derive(Clone, Debug)]
pub struct VerifiedOptimizedProgram {
    verified: VerifiedProgram,
    certificate: OptimizationCertificate,
    stats: OptimizationStats,
}

impl PartialEq for VerifiedOptimizedProgram {
    fn eq(&self, other: &Self) -> bool {
        exact_program_equal(self.program(), other.program())
            && self.certificate == other.certificate
            && self.stats == other.stats
    }
}

impl VerifiedOptimizedProgram {
    pub fn program(&self) -> &Program {
        self.verified.program()
    }

    pub fn verified_program(&self) -> &VerifiedProgram {
        &self.verified
    }

    pub fn certificate(&self) -> &OptimizationCertificate {
        &self.certificate
    }

    pub const fn stats(&self) -> &OptimizationStats {
        &self.stats
    }
}

/// Discover deterministic edits, build a private candidate, and submit both to
/// the independent certificate boundary.
pub fn optimize(
    input: &VerifiedProgram,
    limits: OptimizationLimits,
) -> Result<VerifiedOptimizedProgram, OptimizationError> {
    verify(input.program().clone()).map_err(|error| {
        OptimizationError::new(
            OptimizationFailureCode::InputVerification,
            error.to_string(),
        )
    })?;
    let mut budget = Budget::new(limits);
    let records = discover_edits(input.program(), &mut budget)?;
    let certificate = OptimizationCertificate { records };
    let candidate = reconstruct_candidate(input.program(), &certificate, &mut budget)?;
    verify_optimization(input, candidate, certificate, limits)
}

/// Verify a certificate independently from edit discovery, reconstruct the
/// exact candidate on a private clone, compare it byte-for-byte at the IR model
/// level, and run ordinary SSA verification again.
pub fn verify_optimization(
    input: &VerifiedProgram,
    candidate: Program,
    certificate: OptimizationCertificate,
    limits: OptimizationLimits,
) -> Result<VerifiedOptimizedProgram, OptimizationError> {
    verify(input.program().clone()).map_err(|error| {
        OptimizationError::new(
            OptimizationFailureCode::InputVerification,
            error.to_string(),
        )
    })?;
    let mut budget = Budget::new(limits);
    budget.charge(instruction_count(input.program()))?;
    let independently_expected = independently_verify_records(input.program(), &mut budget)?;
    if certificate.records != independently_expected {
        return Err(OptimizationError::new(
            OptimizationFailureCode::CertificateMismatch,
            "certificate is missing, stale, reordered, or does not describe the canonical edits",
        ));
    }
    budget.charge_certificate(&certificate)?;
    let reconstructed = reconstruct_candidate(input.program(), &certificate, &mut budget)?;
    if !exact_program_equal(&reconstructed, &candidate) {
        return Err(OptimizationError::new(
            OptimizationFailureCode::CandidateMismatch,
            "candidate does not exactly equal independently reconstructed certified output",
        ));
    }
    let verified = verify(candidate).map_err(|error| {
        OptimizationError::new(
            OptimizationFailureCode::OutputVerification,
            error.to_string(),
        )
    })?;
    let input_instructions = instruction_count(input.program());
    let output_instructions = instruction_count(verified.program());
    let algebraic_rewrites = certificate
        .records
        .iter()
        .filter(|record| record.kind == OptimizationEditKind::AlgebraicIdentity)
        .count() as u64;
    let checked_i64_rewrites = certificate
        .records
        .iter()
        .filter(|record| record.kind == OptimizationEditKind::CheckedI64GlobalValueNumbering)
        .count() as u64;
    let gvn_rewrites = certificate
        .records
        .len()
        .saturating_sub(algebraic_rewrites as usize) as u64;
    let certificate_bytes = certificate_size(&certificate)?;
    let stats = OptimizationStats {
        input_instructions,
        output_instructions,
        work_units: budget.work,
        certificate_records: certificate.records.len() as u64,
        certificate_bytes,
        instruction_growth: output_instructions.saturating_sub(input_instructions),
        iterations: budget.iterations,
        algebraic_rewrites,
        gvn_rewrites,
        checked_i64_rewrites,
        cleanup_removed_instructions: input_instructions.saturating_sub(output_instructions),
    };
    Ok(VerifiedOptimizedProgram {
        verified,
        certificate,
        stats,
    })
}

struct Budget {
    limits: OptimizationLimits,
    work: u64,
    iterations: u64,
    input_instructions: Option<u64>,
}

impl Budget {
    const fn new(limits: OptimizationLimits) -> Self {
        Self {
            limits,
            work: 0,
            iterations: 0,
            input_instructions: None,
        }
    }

    fn charge(&mut self, amount: u64) -> Result<(), OptimizationError> {
        self.work = self.work.checked_add(amount).ok_or_else(budget_error)?;
        if self.work > self.limits.max_work_units {
            return Err(budget_error());
        }
        Ok(())
    }

    fn charge_iteration(&mut self) -> Result<(), OptimizationError> {
        self.iterations = self.iterations.checked_add(1).ok_or_else(budget_error)?;
        if self.iterations > self.limits.max_iterations {
            return Err(budget_error());
        }
        Ok(())
    }

    fn check_growth(&mut self, program: &Program) -> Result<(), OptimizationError> {
        let count = instruction_count(program);
        let input = *self.input_instructions.get_or_insert(count);
        if count.saturating_sub(input) > self.limits.max_instruction_growth {
            return Err(budget_error());
        }
        Ok(())
    }

    fn charge_certificate(
        &mut self,
        certificate: &OptimizationCertificate,
    ) -> Result<(), OptimizationError> {
        let records = certificate.records.len() as u64;
        if records > self.limits.max_certificate_records
            || certificate_size(certificate)? > self.limits.max_certificate_bytes
        {
            return Err(budget_error());
        }
        self.charge(records)
    }
}

fn budget_error() -> OptimizationError {
    OptimizationError::new(
        OptimizationFailureCode::BudgetExceeded,
        "optimization work, certificate, growth, or iteration budget exceeded",
    )
}

fn certificate_size(certificate: &OptimizationCertificate) -> Result<u64, OptimizationError> {
    certificate.records.iter().try_fold(0_u64, |total, record| {
        let operands = u64::try_from(record.expected_operands.len())
            .map_err(|_| budget_error())?
            .checked_mul(4)
            .ok_or_else(budget_error)?;
        total
            .checked_add(40)
            .and_then(|value| value.checked_add(operands))
            .ok_or_else(budget_error)
    })
}

fn exact_program_equal(left: &Program, right: &Program) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    let left_instructions = left
        .functions
        .iter_mut()
        .flat_map(|function| &mut function.blocks)
        .flat_map(|block| &mut block.instructions);
    let right_instructions = right
        .functions
        .iter_mut()
        .flat_map(|function| &mut function.blocks)
        .flat_map(|block| &mut block.instructions);
    for (left, right) in left_instructions.zip(right_instructions) {
        if let (
            InstructionKind::Constant(Constant::F64(left_value)),
            InstructionKind::Constant(Constant::F64(right_value)),
        ) = (&mut left.kind, &mut right.kind)
        {
            if left_value.to_bits() != right_value.to_bits() {
                return false;
            }
            *left_value = 0.0;
            *right_value = 0.0;
        }
    }
    left == right
}

fn instruction_count(program: &Program) -> u64 {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .fold(0_u64, |count, block| {
            count.saturating_add(block.instructions.len() as u64)
        })
}

fn discover_edits(
    program: &Program,
    budget: &mut Budget,
) -> Result<Vec<OptimizationCertificateRecord>, OptimizationError> {
    let mut records = Vec::new();
    for function in &program.functions {
        let dominators = Dominators::compute(function, budget)?;
        let mut blocks: Vec<_> = function.blocks.iter().collect();
        blocks.sort_by_key(|block| {
            (
                dominators.depth(block.id).unwrap_or(usize::MAX),
                block.id.raw(),
            )
        });
        for block in blocks {
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                budget.charge(1)?;
                let edit = identity_edit(function, block.id, instruction, budget)?.or_else(|| {
                    gvn_edit(
                        function,
                        &dominators,
                        block.id,
                        instruction_index,
                        instruction,
                    )
                });
                push_record(
                    &mut records,
                    function.id,
                    block.id,
                    instruction.id,
                    edit,
                    budget,
                )?;
            }
        }
    }
    finish_records(records, budget)
}

fn independently_verify_records(
    program: &Program,
    budget: &mut Budget,
) -> Result<Vec<OptimizationCertificateRecord>, OptimizationError> {
    // This verifier traversal starts only from immutable input facts and uses
    // a stable ordered ID set rather than optimizer discovery's sorted block
    // vector. It consumes neither discovery state nor the proposed candidate.
    let mut records = Vec::new();
    for function in &program.functions {
        let dominators = Dominators::compute(function, budget)?;
        let order: BTreeSet<(usize, u32)> = function
            .blocks
            .iter()
            .map(|block| {
                (
                    dominators.depth(block.id).unwrap_or(usize::MAX),
                    block.id.raw(),
                )
            })
            .collect();
        for (_, raw_block) in order {
            let block_id = BlockId::new(raw_block);
            let block = function
                .blocks
                .iter()
                .find(|block| block.id == block_id)
                .ok_or_else(|| {
                    OptimizationError::new(
                        OptimizationFailureCode::InputVerification,
                        "stable verifier block order lost an input block",
                    )
                })?;
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                budget.charge(1)?;
                let edit = identity_edit(function, block.id, instruction, budget)?.or_else(|| {
                    gvn_edit(
                        function,
                        &dominators,
                        block.id,
                        instruction_index,
                        instruction,
                    )
                });
                push_record(
                    &mut records,
                    function.id,
                    block.id,
                    instruction.id,
                    edit,
                    budget,
                )?;
            }
        }
    }
    finish_records(records, budget)
}

fn push_record(
    records: &mut Vec<OptimizationCertificateRecord>,
    function: FunctionId,
    block: BlockId,
    value: ValueId,
    edit: Option<LegalEdit>,
    budget: &Budget,
) -> Result<(), OptimizationError> {
    if let Some((kind, operation, operands, replacement)) = edit {
        let sequence = records.len() as u64;
        records.push(OptimizationCertificateRecord {
            sequence,
            function,
            block,
            value,
            kind,
            expected_operation: operation,
            expected_operands: operands,
            replacement,
        });
        if records.len() as u64 > budget.limits.max_certificate_records {
            return Err(budget_error());
        }
    }
    Ok(())
}

fn finish_records(
    records: Vec<OptimizationCertificateRecord>,
    budget: &Budget,
) -> Result<Vec<OptimizationCertificateRecord>, OptimizationError> {
    let certificate = OptimizationCertificate {
        records: records.clone(),
    };
    if certificate_size(&certificate)? > budget.limits.max_certificate_bytes {
        return Err(budget_error());
    }
    Ok(records)
}

type LegalEdit = (OptimizationEditKind, RuntimeOp, Vec<ValueId>, ValueId);

fn identity_edit(
    function: &Function,
    _block: BlockId,
    instruction: &Instruction,
    budget: &mut Budget,
) -> Result<Option<LegalEdit>, OptimizationError> {
    let InstructionKind::Runtime {
        operation,
        arguments,
        ..
    } = &instruction.kind
    else {
        return Ok(None);
    };
    if instruction.metadata.safepoint != Safepoint::None
        || instruction.metadata.frame_state.is_some()
        || instruction.metadata.effects != EffectSet::PURE
        || instruction.metadata.failure != FailureBehavior::None
    {
        return Ok(None);
    }
    budget.charge(arguments.len() as u64)?;
    let replacement = match (*operation, arguments.as_slice()) {
        (RuntimeOp::BitXor | RuntimeOp::BitOr, [left, right]) => {
            if constant_i64(function, *left) == Some(0) {
                Some(*right)
            } else if constant_i64(function, *right) == Some(0)
                || operation == &RuntimeOp::BitOr && left == right
            {
                Some(*left)
            } else {
                None
            }
        }
        (RuntimeOp::BitAnd, [left, right]) => {
            if constant_i64(function, *left) == Some(-1) {
                Some(*right)
            } else if constant_i64(function, *right) == Some(-1) || left == right {
                Some(*left)
            } else {
                None
            }
        }
        (RuntimeOp::Not, [inner]) => instruction_by_id(function, *inner).and_then(|inner| {
            if inner.ty != SsaType::Bool
                || inner.metadata.effects != EffectSet::PURE
                || inner.metadata.safepoint != Safepoint::None
                || inner.metadata.frame_state.is_some()
            {
                return None;
            }
            match &inner.kind {
                InstructionKind::Runtime {
                    operation: RuntimeOp::Not,
                    arguments,
                    ..
                } if arguments.len() == 1 => arguments.first().copied(),
                _ => None,
            }
        }),
        _ => None,
    };
    Ok(replacement.map(|replacement| {
        (
            OptimizationEditKind::AlgebraicIdentity,
            *operation,
            arguments.clone(),
            replacement,
        )
    }))
}

fn gvn_edit(
    function: &Function,
    dominators: &Dominators,
    block: BlockId,
    instruction_index: usize,
    instruction: &Instruction,
) -> Option<LegalEdit> {
    if !gvn_eligible(function, instruction) {
        return None;
    }
    let InstructionKind::Runtime {
        operation,
        arguments,
        signature,
    } = &instruction.kind
    else {
        return None;
    };
    let mut candidates = function
        .blocks
        .iter()
        .flat_map(|candidate_block| {
            candidate_block
                .instructions
                .iter()
                .enumerate()
                .map(move |(index, candidate)| (candidate_block.id, index, candidate))
        })
        .filter(|(candidate_block, candidate_index, candidate)| {
            candidate.id != instruction.id
                && gvn_eligible(function, candidate)
                && match &candidate.kind {
                    InstructionKind::Runtime {
                        operation: candidate_operation,
                        arguments: candidate_arguments,
                        signature: candidate_signature,
                    } => {
                        candidate_operation == operation
                            && candidate_arguments == arguments
                            && candidate_signature == signature
                            && candidate.ty == instruction.ty
                    }
                    _ => false,
                }
                && dominators.definition_dominates(
                    *candidate_block,
                    *candidate_index,
                    block,
                    instruction_index,
                )
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(candidate_block, index, candidate)| {
        (candidate_block.raw(), *index, candidate.id.raw())
    });
    let (_, _, candidate) = candidates.first().copied()?;
    let kind = if checked_i64_operation(instruction, *operation) {
        OptimizationEditKind::CheckedI64GlobalValueNumbering
    } else {
        OptimizationEditKind::GlobalValueNumbering
    };
    Some((kind, *operation, arguments.clone(), candidate.id))
}

fn gvn_eligible(function: &Function, instruction: &Instruction) -> bool {
    if instruction.metadata.safepoint != Safepoint::None
        || instruction.metadata.frame_state.is_some()
        || !is_scalar(&instruction.ty)
    {
        return false;
    }
    let InstructionKind::Runtime {
        operation,
        arguments,
        ..
    } = &instruction.kind
    else {
        return false;
    };
    if !arguments
        .iter()
        .all(|argument| value_type(function, *argument).is_some_and(is_scalar))
    {
        return false;
    }
    let pure = matches!(
        operation,
        RuntimeOp::Less
            | RuntimeOp::LessEqual
            | RuntimeOp::Greater
            | RuntimeOp::GreaterEqual
            | RuntimeOp::Not
            | RuntimeOp::BitAnd
            | RuntimeOp::BitOr
            | RuntimeOp::BitXor
    ) && instruction.metadata.effects == EffectSet::PURE
        && instruction.metadata.failure == FailureBehavior::None;
    let checked = matches!(
        operation,
        RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide
    ) && instruction.metadata.effects == EffectSet::MAY_TRAP
        && instruction.metadata.failure == FailureBehavior::Trap;
    pure || checked
}

fn checked_i64_operation(instruction: &Instruction, operation: RuntimeOp) -> bool {
    instruction.ty == SsaType::I64
        && matches!(
            operation,
            RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide
        )
}

fn is_scalar(ty: &SsaType) -> bool {
    matches!(
        ty,
        SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
    )
}

fn value_type(function: &Function, value: ValueId) -> Option<&SsaType> {
    function.blocks.iter().find_map(|block| {
        block
            .parameters
            .iter()
            .find(|parameter| parameter.id == value)
            .map(|parameter| &parameter.ty)
            .or_else(|| {
                block
                    .instructions
                    .iter()
                    .find(|instruction| instruction.id == value)
                    .map(|instruction| &instruction.ty)
            })
    })
}

fn instruction_by_id(function: &Function, value: ValueId) -> Option<&Instruction> {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find(|instruction| instruction.id == value)
}

fn constant_i64(function: &Function, value: ValueId) -> Option<i64> {
    match &instruction_by_id(function, value)?.kind {
        InstructionKind::Constant(Constant::I64(value)) => Some(*value),
        _ => None,
    }
}

fn reconstruct_candidate(
    input: &Program,
    certificate: &OptimizationCertificate,
    budget: &mut Budget,
) -> Result<Program, OptimizationError> {
    budget.input_instructions = Some(instruction_count(input));
    let mut candidate = input.clone();
    for (sequence, record) in certificate.records.iter().enumerate() {
        budget.charge(1_u64.saturating_add(record.expected_operands.len() as u64))?;
        if record.sequence != sequence as u64 {
            return Err(OptimizationError::new(
                OptimizationFailureCode::IllegalEdit,
                "certificate sequence is not dense and ordered",
            ));
        }
        let function = candidate
            .functions
            .get_mut(record.function.index().unwrap_or(usize::MAX))
            .filter(|function| function.id == record.function)
            .ok_or_else(|| {
                OptimizationError::new(
                    OptimizationFailureCode::IllegalEdit,
                    "certificate function ID is stale",
                )
            })?;
        let block = function
            .blocks
            .iter_mut()
            .find(|block| block.id == record.block)
            .ok_or_else(|| {
                OptimizationError::new(
                    OptimizationFailureCode::IllegalEdit,
                    "certificate block ID is stale",
                )
            })?;
        let instruction = block
            .instructions
            .iter_mut()
            .find(|instruction| instruction.id == record.value)
            .ok_or_else(|| {
                OptimizationError::new(
                    OptimizationFailureCode::IllegalEdit,
                    "certificate value ID is stale",
                )
            })?;
        match &instruction.kind {
            InstructionKind::Runtime {
                operation,
                arguments,
                ..
            } if *operation == record.expected_operation
                && *arguments == record.expected_operands => {}
            _ => {
                return Err(OptimizationError::new(
                    OptimizationFailureCode::IllegalEdit,
                    "certificate operation or operands do not match the verified input",
                ));
            }
        }
        instruction.kind = InstructionKind::Copy(record.replacement);
        instruction.metadata.effects = EffectSet::PURE;
        instruction.metadata.safepoint = Safepoint::None;
        instruction.metadata.failure = FailureBehavior::None;
        instruction.metadata.frame_state = None;
    }
    budget.check_growth(&candidate)?;
    let mut verified = verify(candidate).map_err(|error| {
        OptimizationError::new(OptimizationFailureCode::IllegalEdit, error.to_string())
    })?;
    macro_rules! cleanup {
        ($pass:ident) => {{
            budget.charge_iteration()?;
            budget.charge(instruction_count(verified.program()))?;
            verified = $pass(&verified).map_err(|error| {
                OptimizationError::new(
                    OptimizationFailureCode::OutputVerification,
                    error.to_string(),
                )
            })?;
            budget.check_growth(verified.program())?;
        }};
    }
    cleanup!(copy_propagate);
    cleanup!(simplify_branches);
    cleanup!(unreachable_blocks);
    cleanup!(empty_block_forwarding);
    cleanup!(effect_aware_dce);
    cleanup!(direct_call_resolution);
    cleanup!(canonical_block_order);
    budget.charge(instruction_count(verified.program()))?;
    Ok(verified.into_program())
}

struct Dominators {
    sets: Vec<Vec<bool>>,
}

impl Dominators {
    fn compute(function: &Function, budget: &mut Budget) -> Result<Self, OptimizationError> {
        let count = function.blocks.len();
        let entry = function.entry.index().ok_or_else(|| {
            OptimizationError::new(
                OptimizationFailureCode::InputVerification,
                "entry block ID cannot index verified function",
            )
        })?;
        let mut predecessors = vec![BTreeSet::new(); count];
        for block in &function.blocks {
            for successor in successors(&block.terminator) {
                let successor = successor.index().ok_or_else(|| {
                    OptimizationError::new(
                        OptimizationFailureCode::InputVerification,
                        "successor block ID cannot index verified function",
                    )
                })?;
                let Some(set) = predecessors.get_mut(successor) else {
                    return Err(OptimizationError::new(
                        OptimizationFailureCode::InputVerification,
                        "successor block is outside verified function",
                    ));
                };
                set.insert(block.id.index().unwrap_or(usize::MAX));
                budget.charge(1)?;
            }
        }
        let mut sets = vec![vec![true; count]; count];
        if let Some(entry_set) = sets.get_mut(entry) {
            entry_set.fill(false);
            if let Some(value) = entry_set.get_mut(entry) {
                *value = true;
            }
        }
        loop {
            let mut changed = false;
            for block in 0..count {
                if block == entry {
                    continue;
                }
                let mut next = vec![true; count];
                let Some(preds) = predecessors.get(block) else {
                    continue;
                };
                for predecessor in preds {
                    let Some(predecessor_set) = sets.get(*predecessor) else {
                        continue;
                    };
                    for (value, predecessor_value) in next.iter_mut().zip(predecessor_set) {
                        *value &= *predecessor_value;
                        budget.charge(1)?;
                    }
                }
                if let Some(value) = next.get_mut(block) {
                    *value = true;
                }
                if sets.get(block) != Some(&next) {
                    sets[block] = next;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Ok(Self { sets })
    }

    fn depth(&self, block: BlockId) -> Option<usize> {
        self.sets
            .get(block.index()?)
            .map(|set| set.iter().filter(|value| **value).count())
    }

    fn definition_dominates(
        &self,
        definition_block: BlockId,
        definition_index: usize,
        use_block: BlockId,
        use_index: usize,
    ) -> bool {
        if definition_block == use_block {
            return definition_index < use_index;
        }
        let Some(definition) = definition_block.index() else {
            return false;
        };
        use_block
            .index()
            .and_then(|block| self.sets.get(block))
            .and_then(|set| set.get(definition))
            .copied()
            .unwrap_or(false)
    }
}

fn successors(terminator: &crate::Terminator) -> Vec<BlockId> {
    match terminator {
        crate::Terminator::Branch { target, .. } => vec![*target],
        crate::Terminator::ConditionalBranch {
            true_target,
            false_target,
            ..
        } => vec![*true_target, *false_target],
        _ => Vec::new(),
    }
}
