use std::collections::{HashMap, VecDeque};
use std::fmt;

use crate::{
    canonical_block_order, copy_propagate, direct_call_resolution, effect_aware_dce,
    empty_block_forwarding, simplify_branches, unreachable_blocks, verify, BlockId, Constant,
    EffectSet, FailureBehavior, Function, FunctionId, Instruction, InstructionKind, Program,
    RuntimeOp, Safepoint, Signature, SsaType, ValueId, VerifiedProgram,
};

/// Deterministic resource bounds for the proof-producing optimization slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptimizationLimits {
    pub max_work_units: u64,
    pub max_certificate_records: u64,
    pub max_certificate_bytes_estimate: u64,
    pub max_instruction_growth: u64,
    pub max_iterations: u64,
    pub max_functions: u64,
    pub max_blocks: u64,
    pub max_parameters: u64,
    pub max_instructions: u64,
    pub max_operands: u64,
    pub max_frame_facts: u64,
    pub max_type_nodes: u64,
    pub max_metadata_items: u64,
    pub max_string_and_metadata_bytes: u64,
}

impl Default for OptimizationLimits {
    fn default() -> Self {
        Self {
            max_work_units: 16 * 1024 * 1024,
            max_certificate_records: 65_536,
            max_certificate_bytes_estimate: 4 * 1024 * 1024,
            max_instruction_growth: 0,
            // Internal optimize performs the seven cleanup passes once while
            // constructing and once while independently checking the candidate.
            max_iterations: 16,
            max_functions: 4_096,
            max_blocks: 65_536,
            max_parameters: 1_048_576,
            max_instructions: 1_048_576,
            max_operands: 4_194_304,
            max_frame_facts: 1_048_576,
            max_type_nodes: 4_194_304,
            max_metadata_items: 4_194_304,
            max_string_and_metadata_bytes: 16 * 1024 * 1024,
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
/// Operation and operands are repeated deliberately: the certificate checker
/// checks them against its private immutable input indexes rather than trusting
/// edit discovery.
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
    pub certificate_bytes_estimate: u64,
    pub instruction_growth: u64,
    pub iterations: u64,
    pub discovery_passes: u64,
    pub checker_passes: u64,
    pub reconstruction_passes: u64,
    pub cleanup_passes: u64,
    pub validation_passes: u64,
    pub optimizing_passes: u64,
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
/// the independent certificate boundary under one aggregate budget.
pub fn optimize(
    input: &VerifiedProgram,
    limits: OptimizationLimits,
) -> Result<VerifiedOptimizedProgram, OptimizationError> {
    let mut budget = Budget::new(limits);
    let input_shape = preflight_program(input.program(), &mut budget)?;
    budget.set_input_instructions(input_shape.instructions);

    budget.discovery_passes = budget.discovery_passes.saturating_add(1);
    let discovery = DiscoveryIndexes::build(input.program(), &input_shape, &mut budget)?;
    let records = discover_edits(input.program(), &discovery, &mut budget)?;
    let certificate = OptimizationCertificate { records };
    let candidate = discovery_reconstruct(input.program(), &certificate, &discovery, &mut budget)?;
    let candidate_shape = preflight_program(&candidate, &mut budget)?;
    budget.check_growth(candidate_shape.instructions)?;

    verify_optimization_with_budget(
        input,
        candidate,
        candidate_shape,
        certificate,
        input_shape,
        &mut budget,
    )
}

/// Independently verify a certificate, reconstruct the exact candidate on a
/// private clone, compare by reference at the IR model level, and run ordinary
/// SSA verification again.
pub fn verify_optimization(
    input: &VerifiedProgram,
    candidate: Program,
    certificate: OptimizationCertificate,
    limits: OptimizationLimits,
) -> Result<VerifiedOptimizedProgram, OptimizationError> {
    let mut budget = Budget::new(limits);
    // Candidate preflight is deliberately first: an untrusted oversized public
    // candidate is rejected before cloning, input traversal, or SSA verification.
    let candidate_shape = preflight_program(&candidate, &mut budget)?;
    let input_shape = preflight_program(input.program(), &mut budget)?;
    budget.set_input_instructions(input_shape.instructions);
    budget.check_growth(candidate_shape.instructions)?;
    verify_optimization_with_budget(
        input,
        candidate,
        candidate_shape,
        certificate,
        input_shape,
        &mut budget,
    )
}

fn verify_optimization_with_budget(
    input: &VerifiedProgram,
    candidate: Program,
    candidate_shape: ProgramShape,
    certificate: OptimizationCertificate,
    input_shape: ProgramShape,
    budget: &mut Budget,
) -> Result<VerifiedOptimizedProgram, OptimizationError> {
    preflight_certificate(&certificate, budget)?;
    budget.checker_passes = budget.checker_passes.saturating_add(1);
    let checker = CheckerIndexes::build(input.program(), &input_shape, budget)?;
    checker.verify_records(input.program(), &certificate, budget)?;
    let reconstructed = checker_reconstruct(input.program(), &certificate, &checker, budget)?;
    let reconstructed_shape = preflight_program(&reconstructed, budget)?;
    budget.check_growth(reconstructed_shape.instructions)?;
    budget.charge(candidate_shape.comparison_units())?;
    if !exact_program_equal(&reconstructed, &candidate) {
        return Err(OptimizationError::new(
            OptimizationFailureCode::CandidateMismatch,
            "candidate does not exactly equal independently reconstructed certified output",
        ));
    }

    budget.charge_validation(&candidate_shape)?;
    let verified = verify(candidate).map_err(|error| {
        OptimizationError::new(
            OptimizationFailureCode::OutputVerification,
            error.to_string(),
        )
    })?;
    let output_instructions = candidate_shape.instructions;
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
    let certificate_bytes_estimate = certificate_size_estimate(&certificate)?;
    let optimizing_passes = budget
        .discovery_passes
        .saturating_add(budget.checker_passes)
        .saturating_add(budget.reconstruction_passes)
        .saturating_add(budget.cleanup_passes)
        .saturating_add(budget.validation_passes);
    let stats = OptimizationStats {
        input_instructions: input_shape.instructions,
        output_instructions,
        work_units: budget.work,
        certificate_records: certificate.records.len() as u64,
        certificate_bytes_estimate,
        instruction_growth: output_instructions.saturating_sub(input_shape.instructions),
        iterations: budget.iterations,
        discovery_passes: budget.discovery_passes,
        checker_passes: budget.checker_passes,
        reconstruction_passes: budget.reconstruction_passes,
        cleanup_passes: budget.cleanup_passes,
        validation_passes: budget.validation_passes,
        optimizing_passes,
        algebraic_rewrites,
        gvn_rewrites,
        checked_i64_rewrites,
        cleanup_removed_instructions: input_shape.instructions.saturating_sub(output_instructions),
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
    discovery_passes: u64,
    checker_passes: u64,
    reconstruction_passes: u64,
    cleanup_passes: u64,
    validation_passes: u64,
}

impl Budget {
    const fn new(limits: OptimizationLimits) -> Self {
        Self {
            limits,
            work: 0,
            iterations: 0,
            input_instructions: None,
            discovery_passes: 0,
            checker_passes: 0,
            reconstruction_passes: 0,
            cleanup_passes: 0,
            validation_passes: 0,
        }
    }

    fn charge(&mut self, amount: u64) -> Result<(), OptimizationError> {
        self.work = self.work.checked_add(amount).ok_or_else(budget_error)?;
        if self.work > self.limits.max_work_units {
            return Err(budget_error());
        }
        Ok(())
    }

    fn set_input_instructions(&mut self, instructions: u64) {
        self.input_instructions = Some(instructions);
    }

    fn charge_cleanup_pass(&mut self) -> Result<(), OptimizationError> {
        self.iterations = self.iterations.checked_add(1).ok_or_else(budget_error)?;
        if self.iterations > self.limits.max_iterations {
            return Err(budget_error());
        }
        self.cleanup_passes = self.cleanup_passes.saturating_add(1);
        Ok(())
    }

    fn check_growth(&self, instructions: u64) -> Result<(), OptimizationError> {
        let input = self.input_instructions.unwrap_or(instructions);
        if instructions.saturating_sub(input) > self.limits.max_instruction_growth {
            return Err(budget_error());
        }
        Ok(())
    }

    fn charge_validation(&mut self, shape: &ProgramShape) -> Result<(), OptimizationError> {
        self.validation_passes = self.validation_passes.saturating_add(1);
        self.charge(shape.validation_units())
    }
}

fn budget_error() -> OptimizationError {
    OptimizationError::new(
        OptimizationFailureCode::BudgetExceeded,
        "optimization work, shape, certificate, growth, or iteration budget exceeded",
    )
}

const CERTIFICATE_HEADER_BYTES_ESTIMATE: u64 = 8;
const CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE: u64 = 31;

fn certificate_size_estimate(
    certificate: &OptimizationCertificate,
) -> Result<u64, OptimizationError> {
    certificate
        .records
        .iter()
        .try_fold(CERTIFICATE_HEADER_BYTES_ESTIMATE, |total, record| {
            let operands = u64::try_from(record.expected_operands.len())
                .map_err(|_| budget_error())?
                .checked_mul(4)
                .ok_or_else(budget_error)?;
            total
                .checked_add(CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE)
                .and_then(|value| value.checked_add(operands))
                .ok_or_else(budget_error)
        })
}

fn preflight_certificate(
    certificate: &OptimizationCertificate,
    budget: &mut Budget,
) -> Result<(), OptimizationError> {
    let records = u64::try_from(certificate.records.len()).map_err(|_| budget_error())?;
    if records > budget.limits.max_certificate_records {
        return Err(budget_error());
    }
    let bytes = certificate_size_estimate(certificate)?;
    if bytes > budget.limits.max_certificate_bytes_estimate {
        return Err(budget_error());
    }
    budget.charge(CERTIFICATE_HEADER_BYTES_ESTIMATE)?;
    for record in &certificate.records {
        let operands = u64::try_from(record.expected_operands.len()).map_err(|_| budget_error())?;
        budget.charge(
            CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE
                .checked_add(operands)
                .ok_or_else(budget_error)?,
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
struct ProgramShape {
    functions: u64,
    blocks: u64,
    parameters: u64,
    instructions: u64,
    operands: u64,
    frame_facts: u64,
    type_nodes: u64,
    metadata_items: u64,
    string_and_metadata_bytes: u64,
}

impl ProgramShape {
    fn allocation_units(self) -> u64 {
        self.functions
            .saturating_add(self.blocks)
            .saturating_add(self.parameters)
            .saturating_add(self.instructions)
            .saturating_add(self.operands)
            .saturating_add(self.frame_facts)
            .saturating_add(self.type_nodes)
            .saturating_add(self.metadata_items)
            .saturating_add(self.string_and_metadata_bytes)
    }

    fn comparison_units(self) -> u64 {
        self.allocation_units()
    }

    fn validation_units(self) -> u64 {
        let words = self.blocks.saturating_add(63) / 64;
        self.allocation_units()
            .saturating_add(self.blocks.saturating_mul(words))
            .saturating_add(self.operands)
    }
}

struct ShapeCounter<'a> {
    shape: ProgramShape,
    limits: OptimizationLimits,
    budget: &'a mut Budget,
}

impl<'a> ShapeCounter<'a> {
    fn new(budget: &'a mut Budget) -> Self {
        Self {
            shape: ProgramShape::default(),
            limits: budget.limits,
            budget,
        }
    }

    fn add_bounded(&mut self, field: ShapeField, amount: u64) -> Result<(), OptimizationError> {
        let (slot, limit) = match field {
            ShapeField::Functions => (&mut self.shape.functions, self.limits.max_functions),
            ShapeField::Blocks => (&mut self.shape.blocks, self.limits.max_blocks),
            ShapeField::Parameters => (&mut self.shape.parameters, self.limits.max_parameters),
            ShapeField::Instructions => {
                (&mut self.shape.instructions, self.limits.max_instructions)
            }
            ShapeField::Operands => (&mut self.shape.operands, self.limits.max_operands),
            ShapeField::FrameFacts => (&mut self.shape.frame_facts, self.limits.max_frame_facts),
            ShapeField::TypeNodes => (&mut self.shape.type_nodes, self.limits.max_type_nodes),
            ShapeField::MetadataItems => (
                &mut self.shape.metadata_items,
                self.limits.max_metadata_items,
            ),
            ShapeField::StringAndMetadataBytes => (
                &mut self.shape.string_and_metadata_bytes,
                self.limits.max_string_and_metadata_bytes,
            ),
        };
        *slot = slot.checked_add(amount).ok_or_else(budget_error)?;
        if *slot > limit {
            return Err(budget_error());
        }
        self.budget.charge(amount)
    }

    fn add_string(&mut self, value: &str) -> Result<(), OptimizationError> {
        self.add_bounded(
            ShapeField::StringAndMetadataBytes,
            u64::try_from(value.len()).map_err(|_| budget_error())?,
        )
    }

    fn add_metadata(&mut self) -> Result<(), OptimizationError> {
        self.add_bounded(ShapeField::MetadataItems, 1)?;
        self.add_bounded(ShapeField::StringAndMetadataBytes, 8)
    }

    fn add_signature(&mut self, signature: &Signature) -> Result<(), OptimizationError> {
        self.add_metadata()?;
        self.add_bounded(
            ShapeField::Parameters,
            u64::try_from(signature.parameters.len()).map_err(|_| budget_error())?,
        )?;
        self.add_bounded(
            ShapeField::MetadataItems,
            u64::try_from(signature.type_parameters.len()).map_err(|_| budget_error())?,
        )?;
        for name in &signature.type_parameters {
            self.add_string(name)?;
        }
        for bound in &signature.bounds {
            self.add_string(&bound.parameter)?;
            self.add_metadata()?;
        }
        for ty in &signature.parameters {
            self.add_type(ty)?;
        }
        self.add_type(&signature.result)
    }

    fn add_type(&mut self, root: &SsaType) -> Result<(), OptimizationError> {
        let mut pending = vec![root];
        while let Some(ty) = pending.pop() {
            self.add_bounded(ShapeField::TypeNodes, 1)?;
            self.add_bounded(ShapeField::StringAndMetadataBytes, 1)?;
            match ty {
                SsaType::Owned(inner)
                | SsaType::Ref(inner)
                | SsaType::RefMut(inner)
                | SsaType::List(inner)
                | SsaType::Option(inner) => pending.push(inner),
                SsaType::Result(ok, error) => {
                    pending.push(ok);
                    pending.push(error);
                }
                SsaType::Function(signature) => {
                    self.add_metadata()?;
                    self.add_bounded(
                        ShapeField::Parameters,
                        u64::try_from(signature.parameters.len()).map_err(|_| budget_error())?,
                    )?;
                    self.add_bounded(
                        ShapeField::MetadataItems,
                        u64::try_from(signature.type_parameters.len())
                            .map_err(|_| budget_error())?,
                    )?;
                    for name in &signature.type_parameters {
                        self.add_string(name)?;
                    }
                    for bound in &signature.bounds {
                        self.add_string(&bound.parameter)?;
                        self.add_metadata()?;
                    }
                    for parameter in &signature.parameters {
                        pending.push(parameter);
                    }
                    pending.push(&signature.result);
                }
                SsaType::TypeParameter(name) => self.add_string(name)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn add_frame(&mut self, frame: &crate::FrameState) -> Result<(), OptimizationError> {
        let facts = 1_u64
            .checked_add(u64::try_from(frame.locals.len()).map_err(|_| budget_error())?)
            .and_then(|value| value.checked_add(frame.operand_stack.len() as u64))
            .ok_or_else(budget_error)?;
        self.add_bounded(ShapeField::FrameFacts, facts)?;
        self.add_bounded(ShapeField::StringAndMetadataBytes, facts.saturating_mul(16))
    }

    fn add_instruction(&mut self, instruction: &Instruction) -> Result<(), OptimizationError> {
        self.add_bounded(ShapeField::Instructions, 1)?;
        self.add_metadata()?;
        self.add_type(&instruction.ty)?;
        if let Some(frame) = &instruction.metadata.frame_state {
            self.add_frame(frame)?;
        }
        let operands = instruction_operand_count(&instruction.kind)?;
        self.add_bounded(ShapeField::Operands, operands)?;
        match &instruction.kind {
            InstructionKind::Constant(Constant::Str(value) | Constant::Symbol(value)) => {
                self.add_string(value)?
            }
            InstructionKind::Runtime { signature, .. }
            | InstructionKind::Call { signature, .. } => {
                self.add_signature(signature)?;
                if let InstructionKind::Call {
                    instantiation: Some(instantiation),
                    ..
                } = &instruction.kind
                {
                    for substitution in &instantiation.substitutions {
                        self.add_string(&substitution.parameter)?;
                        self.add_type(&substitution.ty)?;
                    }
                    for witness in &instantiation.witnesses {
                        self.add_type(&witness.ty)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ShapeField {
    Functions,
    Blocks,
    Parameters,
    Instructions,
    Operands,
    FrameFacts,
    TypeNodes,
    MetadataItems,
    StringAndMetadataBytes,
}

fn preflight_program(
    program: &Program,
    budget: &mut Budget,
) -> Result<ProgramShape, OptimizationError> {
    let mut counter = ShapeCounter::new(budget);
    counter.add_bounded(
        ShapeField::Functions,
        u64::try_from(program.functions.len()).map_err(|_| budget_error())?,
    )?;
    let top_metadata = program
        .sources
        .len()
        .checked_add(program.products.len())
        .and_then(|value| value.checked_add(program.traits.len()))
        .and_then(|value| value.checked_add(program.implementations.len()))
        .ok_or_else(budget_error)?;
    counter.add_bounded(
        ShapeField::MetadataItems,
        u64::try_from(top_metadata).map_err(|_| budget_error())?,
    )?;
    counter.add_bounded(
        ShapeField::StringAndMetadataBytes,
        u64::try_from(top_metadata)
            .map_err(|_| budget_error())?
            .saturating_mul(8),
    )?;
    for source in &program.sources {
        counter.add_string(&source.path)?;
    }
    for product in &program.products {
        counter.add_string(&product.name)?;
        for field in &product.fields {
            counter.add_metadata()?;
            counter.add_string(&field.name)?;
            counter.add_type(&field.ty)?;
        }
    }
    for trait_metadata in &program.traits {
        counter.add_string(&trait_metadata.name)?;
    }
    for function in &program.functions {
        counter.add_string(&function.name)?;
        counter.add_signature(&function.signature)?;
        for place in &function.places {
            counter.add_metadata()?;
            counter.add_type(&place.ty)?;
        }
        counter.add_bounded(
            ShapeField::Blocks,
            u64::try_from(function.blocks.len()).map_err(|_| budget_error())?,
        )?;
        for block in &function.blocks {
            counter.add_metadata()?;
            if let Some(frame) = &block.metadata.frame_state {
                counter.add_frame(frame)?;
            }
            counter.add_bounded(
                ShapeField::Parameters,
                u64::try_from(block.parameters.len()).map_err(|_| budget_error())?,
            )?;
            for parameter in &block.parameters {
                counter.add_metadata()?;
                counter.add_type(&parameter.ty)?;
            }
            for instruction in &block.instructions {
                counter.add_instruction(instruction)?;
            }
            counter.add_bounded(
                ShapeField::Operands,
                terminator_operand_count(&block.terminator)?,
            )?;
            if let crate::Terminator::Trap { message } = &block.terminator {
                counter.add_string(message)?;
            }
        }
    }
    Ok(counter.shape)
}

fn instruction_operand_count(kind: &InstructionKind) -> Result<u64, OptimizationError> {
    let count = match kind {
        InstructionKind::Constant(_)
        | InstructionKind::PlaceEnd { .. }
        | InstructionKind::FunctionRef(_) => 0,
        InstructionKind::Copy(_)
        | InstructionKind::PlaceInit { .. }
        | InstructionKind::Move { .. }
        | InstructionKind::Borrow { .. }
        | InstructionKind::ProductField { .. } => 1,
        InstructionKind::Runtime { arguments, .. }
        | InstructionKind::Call {
            target: crate::CallTarget::Direct(_),
            arguments,
            ..
        } => arguments.len(),
        InstructionKind::Call {
            target: crate::CallTarget::Indirect(_),
            arguments,
            ..
        } => arguments.len().checked_add(1).ok_or_else(budget_error)?,
        InstructionKind::ProductValue { fields, .. } => fields.len(),
        InstructionKind::WithProductField { .. } => 2,
    };
    u64::try_from(count).map_err(|_| budget_error())
}

fn terminator_operand_count(terminator: &crate::Terminator) -> Result<u64, OptimizationError> {
    let count = match terminator {
        crate::Terminator::Branch { arguments, .. } => arguments.len(),
        crate::Terminator::ConditionalBranch {
            true_arguments,
            false_arguments,
            ..
        } => 1_usize
            .checked_add(true_arguments.len())
            .and_then(|value| value.checked_add(false_arguments.len()))
            .ok_or_else(budget_error)?,
        crate::Terminator::Return(_) | crate::Terminator::Exit { .. } => 1,
        crate::Terminator::Trap { .. } => 0,
        crate::Terminator::Outcome { detail, .. } => usize::from(detail.is_some()),
    };
    u64::try_from(count).map_err(|_| budget_error())
}

#[derive(Clone, Copy)]
struct DiscoveryDefinition<'a> {
    block: BlockId,
    instruction_index: Option<usize>,
    ty: &'a SsaType,
    instruction: Option<&'a Instruction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct DiscoveryPosition {
    block: BlockId,
    instruction_index: usize,
    value: ValueId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct DiscoveryExpressionKey<'a> {
    operation: RuntimeOp,
    arguments: &'a [ValueId],
    signature: &'a Signature,
    ty: &'a SsaType,
}

struct DiscoveryFunctionIndexes<'a> {
    definitions: Vec<Option<DiscoveryDefinition<'a>>>,
    constants_i64: Vec<Option<i64>>,
    expressions: HashMap<DiscoveryExpressionKey<'a>, Vec<DiscoveryPosition>>,
    dominance: DiscoveryDominance,
}

struct DiscoveryIndexes<'a> {
    functions: Vec<DiscoveryFunctionIndexes<'a>>,
}

impl<'a> DiscoveryIndexes<'a> {
    fn build(
        program: &'a Program,
        shape: &ProgramShape,
        budget: &mut Budget,
    ) -> Result<Self, OptimizationError> {
        budget.charge(shape.functions)?;
        let mut functions = Vec::with_capacity(program.functions.len());
        for function in &program.functions {
            let value_count = function.blocks.iter().try_fold(0_usize, |total, block| {
                total
                    .checked_add(block.parameters.len())
                    .and_then(|value| value.checked_add(block.instructions.len()))
                    .ok_or_else(budget_error)
            })?;
            budget.charge((value_count as u64).saturating_mul(2))?;
            let mut definitions = vec![None; value_count];
            let mut constants_i64 = vec![None; value_count];
            for block in &function.blocks {
                for parameter in &block.parameters {
                    discovery_insert_definition(
                        &mut definitions,
                        parameter.id,
                        DiscoveryDefinition {
                            block: block.id,
                            instruction_index: None,
                            ty: &parameter.ty,
                            instruction: None,
                        },
                    )?;
                }
                for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                    discovery_insert_definition(
                        &mut definitions,
                        instruction.id,
                        DiscoveryDefinition {
                            block: block.id,
                            instruction_index: Some(instruction_index),
                            ty: &instruction.ty,
                            instruction: Some(instruction),
                        },
                    )?;
                    if let InstructionKind::Constant(Constant::I64(value)) = instruction.kind {
                        let slot = instruction
                            .id
                            .index()
                            .and_then(|index| constants_i64.get_mut(index));
                        if let Some(slot) = slot {
                            *slot = Some(value);
                        }
                    }
                    budget.charge(1)?;
                }
            }
            let dominance = DiscoveryDominance::compute(function, budget)?;
            let mut indexes = DiscoveryFunctionIndexes {
                definitions,
                constants_i64,
                expressions: HashMap::with_capacity(value_count),
                dominance,
            };
            for block in &function.blocks {
                if !indexes.dominance.is_reachable(block.id) {
                    continue;
                }
                for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                    if let Some(key) = discovery_expression_key(&indexes, instruction, budget)? {
                        budget.charge(1)?;
                        indexes
                            .expressions
                            .entry(key)
                            .or_default()
                            .push(DiscoveryPosition {
                                block: block.id,
                                instruction_index,
                                value: instruction.id,
                            });
                    }
                }
            }
            functions.push(indexes);
        }
        Ok(Self { functions })
    }
}

fn discovery_insert_definition<'a>(
    definitions: &mut [Option<DiscoveryDefinition<'a>>],
    value: ValueId,
    definition: DiscoveryDefinition<'a>,
) -> Result<(), OptimizationError> {
    let slot = value
        .index()
        .and_then(|index| definitions.get_mut(index))
        .ok_or_else(|| input_index_error("discovery ValueId is not dense"))?;
    if slot.replace(definition).is_some() {
        return Err(input_index_error("discovery found duplicate ValueId"));
    }
    Ok(())
}

fn input_index_error(detail: &str) -> OptimizationError {
    OptimizationError::new(OptimizationFailureCode::InputVerification, detail)
}

type LegalEdit = (OptimizationEditKind, RuntimeOp, Vec<ValueId>, ValueId);

fn discover_edits(
    program: &Program,
    indexes: &DiscoveryIndexes<'_>,
    budget: &mut Budget,
) -> Result<Vec<OptimizationCertificateRecord>, OptimizationError> {
    let mut builder = CertificateBuilder::new(budget)?;
    for function in &program.functions {
        let function_index = function
            .id
            .index()
            .ok_or_else(|| input_index_error("discovery FunctionId cannot index verified input"))?;
        let index = indexes
            .functions
            .get(function_index)
            .ok_or_else(|| input_index_error("discovery function index is incomplete"))?;
        for block in &function.blocks {
            if !index.dominance.is_reachable(block.id) {
                continue;
            }
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                budget.charge(1)?;
                let edit = match discovery_identity_edit(index, instruction, budget)? {
                    Some(edit) => Some(edit),
                    None => {
                        discovery_gvn_edit(index, block.id, instruction_index, instruction, budget)?
                    }
                };
                builder.push(function.id, block.id, instruction.id, edit, budget)?;
            }
        }
    }
    Ok(builder.finish())
}

struct CertificateBuilder {
    records: Vec<OptimizationCertificateRecord>,
    bytes_estimate: u64,
}

impl CertificateBuilder {
    fn new(budget: &mut Budget) -> Result<Self, OptimizationError> {
        if CERTIFICATE_HEADER_BYTES_ESTIMATE > budget.limits.max_certificate_bytes_estimate {
            return Err(budget_error());
        }
        budget.charge(CERTIFICATE_HEADER_BYTES_ESTIMATE)?;
        Ok(Self {
            records: Vec::new(),
            bytes_estimate: CERTIFICATE_HEADER_BYTES_ESTIMATE,
        })
    }

    fn push(
        &mut self,
        function: FunctionId,
        block: BlockId,
        value: ValueId,
        edit: Option<LegalEdit>,
        budget: &mut Budget,
    ) -> Result<(), OptimizationError> {
        let Some((kind, operation, operands, replacement)) = edit else {
            return Ok(());
        };
        let operand_bytes = (operands.len() as u64)
            .checked_mul(4)
            .ok_or_else(budget_error)?;
        self.bytes_estimate = self
            .bytes_estimate
            .checked_add(CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE)
            .and_then(|bytes| bytes.checked_add(operand_bytes))
            .ok_or_else(budget_error)?;
        if self.records.len() as u64 >= budget.limits.max_certificate_records
            || self.bytes_estimate > budget.limits.max_certificate_bytes_estimate
        {
            return Err(budget_error());
        }
        budget.charge(
            CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE
                .checked_add(operands.len() as u64)
                .ok_or_else(budget_error)?,
        )?;
        self.records.push(OptimizationCertificateRecord {
            sequence: self.records.len() as u64,
            function,
            block,
            value,
            kind,
            expected_operation: operation,
            expected_operands: operands,
            replacement,
        });
        Ok(())
    }

    fn finish(self) -> Vec<OptimizationCertificateRecord> {
        self.records
    }
}

fn discovery_identity_edit(
    indexes: &DiscoveryFunctionIndexes<'_>,
    instruction: &Instruction,
    budget: &mut Budget,
) -> Result<Option<LegalEdit>, OptimizationError> {
    let InstructionKind::Runtime {
        operation,
        arguments,
        signature,
    } = &instruction.kind
    else {
        return Ok(None);
    };
    budget.charge(arguments.len() as u64)?;
    if instruction.metadata.safepoint != Safepoint::None
        || instruction.metadata.frame_state.is_some()
        || instruction.metadata.effects != EffectSet::PURE
        || instruction.metadata.failure != FailureBehavior::None
        || !discovery_signature_matches(indexes, signature, arguments, &instruction.ty)
    {
        return Ok(None);
    }
    let replacement = match (*operation, arguments.as_slice()) {
        (RuntimeOp::BitXor | RuntimeOp::BitOr, [left, right]) if instruction.ty == SsaType::I64 => {
            if discovery_constant_i64(indexes, *left) == Some(0) {
                Some(*right)
            } else if discovery_constant_i64(indexes, *right) == Some(0)
                || operation == &RuntimeOp::BitOr && left == right
            {
                Some(*left)
            } else {
                None
            }
        }
        (RuntimeOp::BitAnd, [left, right]) if instruction.ty == SsaType::I64 => {
            if discovery_constant_i64(indexes, *left) == Some(-1) {
                Some(*right)
            } else if discovery_constant_i64(indexes, *right) == Some(-1) || left == right {
                Some(*left)
            } else {
                None
            }
        }
        (RuntimeOp::Not, [inner]) if instruction.ty == SsaType::Bool => indexes
            .definitions
            .get(inner.index().unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .and_then(|definition| definition.instruction)
            .and_then(|inner_instruction| {
                if inner_instruction.ty != SsaType::Bool
                    || inner_instruction.metadata.effects != EffectSet::PURE
                    || inner_instruction.metadata.failure != FailureBehavior::None
                    || inner_instruction.metadata.safepoint != Safepoint::None
                    || inner_instruction.metadata.frame_state.is_some()
                {
                    return None;
                }
                match &inner_instruction.kind {
                    InstructionKind::Runtime {
                        operation: RuntimeOp::Not,
                        arguments,
                        signature,
                    } if arguments.len() == 1
                        && discovery_signature_matches(
                            indexes,
                            signature,
                            arguments,
                            &inner_instruction.ty,
                        ) =>
                    {
                        arguments.first().copied()
                    }
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

fn discovery_gvn_edit(
    indexes: &DiscoveryFunctionIndexes<'_>,
    block: BlockId,
    instruction_index: usize,
    instruction: &Instruction,
    budget: &mut Budget,
) -> Result<Option<LegalEdit>, OptimizationError> {
    let Some(key) = discovery_expression_key(indexes, instruction, budget)? else {
        return Ok(None);
    };
    let Some(candidates) = indexes.expressions.get(&key) else {
        return Ok(None);
    };
    let mut selected: Option<DiscoveryPosition> = None;
    for candidate in candidates {
        budget.charge(1)?;
        if candidate.value == instruction.id
            || !indexes.dominance.definition_dominates(
                candidate.block,
                candidate.instruction_index,
                block,
                instruction_index,
            )
        {
            continue;
        }
        let key = (
            candidate.block.raw(),
            candidate.instruction_index,
            candidate.value.raw(),
        );
        if selected.is_none_or(|current| {
            key < (
                current.block.raw(),
                current.instruction_index,
                current.value.raw(),
            )
        }) {
            selected = Some(*candidate);
        }
    }
    let Some(candidate) = selected else {
        return Ok(None);
    };
    let InstructionKind::Runtime {
        operation,
        arguments,
        ..
    } = &instruction.kind
    else {
        return Ok(None);
    };
    let kind = if discovery_checked_i64(indexes, instruction, *operation) {
        OptimizationEditKind::CheckedI64GlobalValueNumbering
    } else {
        OptimizationEditKind::GlobalValueNumbering
    };
    Ok(Some((kind, *operation, arguments.clone(), candidate.value)))
}

fn discovery_expression_key<'a>(
    indexes: &DiscoveryFunctionIndexes<'a>,
    instruction: &'a Instruction,
    budget: &mut Budget,
) -> Result<Option<DiscoveryExpressionKey<'a>>, OptimizationError> {
    let InstructionKind::Runtime {
        operation,
        arguments,
        signature,
    } = &instruction.kind
    else {
        return Ok(None);
    };
    budget.charge(arguments.len() as u64)?;
    if instruction.metadata.safepoint != Safepoint::None
        || instruction.metadata.frame_state.is_some()
        || !discovery_scalar(&instruction.ty)
        || !discovery_signature_matches(indexes, signature, arguments, &instruction.ty)
    {
        return Ok(None);
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
    let arithmetic = matches!(
        operation,
        RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide
    ) && instruction.metadata.effects == EffectSet::MAY_TRAP
        && instruction.metadata.failure == FailureBehavior::Trap;
    if !pure && !arithmetic {
        return Ok(None);
    }
    Ok(Some(DiscoveryExpressionKey {
        operation: *operation,
        arguments,
        signature,
        ty: &instruction.ty,
    }))
}

fn discovery_signature_matches(
    indexes: &DiscoveryFunctionIndexes<'_>,
    signature: &Signature,
    arguments: &[ValueId],
    result: &SsaType,
) -> bool {
    signature.type_parameters.is_empty()
        && signature.bounds.is_empty()
        && signature.result.as_ref() == result
        && signature.parameters.len() == arguments.len()
        && signature
            .parameters
            .iter()
            .zip(arguments)
            .all(|(expected, value)| {
                indexes
                    .definitions
                    .get(value.index().unwrap_or(usize::MAX))
                    .and_then(Option::as_ref)
                    .is_some_and(|definition| {
                        definition.ty == expected && discovery_scalar(expected)
                    })
            })
}

fn discovery_constant_i64(indexes: &DiscoveryFunctionIndexes<'_>, value: ValueId) -> Option<i64> {
    indexes.constants_i64.get(value.index()?)?.as_ref().copied()
}

fn discovery_checked_i64(
    indexes: &DiscoveryFunctionIndexes<'_>,
    instruction: &Instruction,
    operation: RuntimeOp,
) -> bool {
    instruction.ty == SsaType::I64
        && matches!(
            operation,
            RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide
        )
        && match &instruction.kind {
            InstructionKind::Runtime { arguments, .. } => arguments.iter().all(|value| {
                indexes
                    .definitions
                    .get(value.index().unwrap_or(usize::MAX))
                    .and_then(Option::as_ref)
                    .is_some_and(|definition| definition.ty == &SsaType::I64)
            }),
            _ => false,
        }
}

fn discovery_scalar(ty: &SsaType) -> bool {
    matches!(
        ty,
        SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
    )
}

struct DiscoveryDominance {
    sets: Vec<Vec<u64>>,
    reachable: Vec<bool>,
}

impl DiscoveryDominance {
    fn compute(function: &Function, budget: &mut Budget) -> Result<Self, OptimizationError> {
        let count = function.blocks.len();
        let words = count.saturating_add(63) / 64;
        budget.charge(
            (count as u64)
                .saturating_mul(words as u64)
                .saturating_add((count as u64).saturating_mul(3)),
        )?;
        let mut predecessors = vec![Vec::new(); count];
        for block in &function.blocks {
            let source = block.id.index().ok_or_else(|| {
                input_index_error("discovery block ID cannot index predecessor table")
            })?;
            for successor in discovery_successors(&block.terminator)
                .into_iter()
                .flatten()
            {
                let target = successor.index().ok_or_else(|| {
                    input_index_error("discovery successor ID cannot index predecessor table")
                })?;
                let list = predecessors.get_mut(target).ok_or_else(|| {
                    input_index_error("discovery successor is outside verified function")
                })?;
                budget.charge(1)?;
                list.push(source);
            }
        }
        let entry = function
            .entry
            .index()
            .ok_or_else(|| input_index_error("discovery entry cannot index verified function"))?;
        let mut reachable = vec![false; count];
        let mut queue = VecDeque::new();
        if let Some(slot) = reachable.get_mut(entry) {
            *slot = true;
            queue.push_back(entry);
        }
        while let Some(block_index) = queue.pop_front() {
            budget.charge(1)?;
            let block = function
                .blocks
                .get(block_index)
                .ok_or_else(|| input_index_error("discovery reachability lost a block"))?;
            for successor in discovery_successors(&block.terminator)
                .into_iter()
                .flatten()
            {
                let successor = successor.index().ok_or_else(|| {
                    input_index_error("discovery reachable successor cannot index")
                })?;
                let slot = reachable.get_mut(successor).ok_or_else(|| {
                    input_index_error("discovery reachable successor is outside function")
                })?;
                if !*slot {
                    *slot = true;
                    queue.push_back(successor);
                }
            }
        }
        let mut sets = vec![vec![0_u64; words]; count];
        for block in 0..count {
            if reachable[block] {
                for candidate in 0..count {
                    if reachable[candidate] {
                        sets[block][candidate / 64] |= 1_u64 << (candidate % 64);
                        budget.charge(1)?;
                    }
                }
            }
            sets[block][block / 64] |= 1_u64 << (block % 64);
        }
        sets[entry].fill(0);
        sets[entry][entry / 64] |= 1_u64 << (entry % 64);
        loop {
            let mut changed = false;
            for block in 0..count {
                if block == entry || !reachable[block] {
                    continue;
                }
                budget.charge(words as u64)?;
                let mut next = vec![u64::MAX; words];
                let mut saw_predecessor = false;
                for predecessor in predecessors[block]
                    .iter()
                    .copied()
                    .filter(|predecessor| reachable[*predecessor])
                {
                    saw_predecessor = true;
                    for (word, predecessor_word) in next.iter_mut().zip(&sets[predecessor]) {
                        *word &= *predecessor_word;
                        budget.charge(1)?;
                    }
                }
                if !saw_predecessor {
                    next.fill(0);
                }
                if let Some(last) = next.last_mut() {
                    let excess = words.saturating_mul(64).saturating_sub(count);
                    if excess > 0 {
                        *last &= u64::MAX >> excess;
                    }
                }
                next[block / 64] |= 1_u64 << (block % 64);
                if sets[block] != next {
                    sets[block] = next;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Ok(Self { sets, reachable })
    }

    fn is_reachable(&self, block: BlockId) -> bool {
        block
            .index()
            .and_then(|index| self.reachable.get(index))
            .copied()
            .unwrap_or(false)
    }

    fn definition_dominates(
        &self,
        definition_block: BlockId,
        definition_index: usize,
        use_block: BlockId,
        use_index: usize,
    ) -> bool {
        if !self.is_reachable(definition_block) || !self.is_reachable(use_block) {
            return false;
        }
        if definition_block == use_block {
            return definition_index < use_index;
        }
        let Some(definition) = definition_block.index() else {
            return false;
        };
        use_block
            .index()
            .and_then(|block| self.sets.get(block))
            .and_then(|set| set.get(definition / 64))
            .is_some_and(|word| word & (1_u64 << (definition % 64)) != 0)
    }
}

fn discovery_successors(terminator: &crate::Terminator) -> [Option<BlockId>; 2] {
    match terminator {
        crate::Terminator::Branch { target, .. } => [Some(*target), None],
        crate::Terminator::ConditionalBranch {
            true_target,
            false_target,
            ..
        } => [Some(*true_target), Some(*false_target)],
        _ => [None, None],
    }
}

// The checker below intentionally duplicates semantic and CFG derivation. It
// must remain independent from discovery helpers and discovery dominators.

#[derive(Clone, Copy)]
struct CheckerDefinition<'a> {
    block: BlockId,
    instruction_index: Option<usize>,
    ty: &'a SsaType,
    instruction: Option<&'a Instruction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct CheckerPosition {
    block: BlockId,
    instruction_index: usize,
    value: ValueId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct CheckerExpressionKey<'a> {
    operation: RuntimeOp,
    arguments: &'a [ValueId],
    signature: &'a Signature,
    result_type: &'a SsaType,
}

struct CheckerFunctionIndexes<'a> {
    definitions: Vec<Option<CheckerDefinition<'a>>>,
    constants: Vec<Option<i64>>,
    expressions: HashMap<CheckerExpressionKey<'a>, Vec<CheckerPosition>>,
    dominance: CheckerDominance,
}

struct CheckerIndexes<'a> {
    functions: Vec<CheckerFunctionIndexes<'a>>,
}

impl<'a> CheckerIndexes<'a> {
    fn build(
        program: &'a Program,
        shape: &ProgramShape,
        budget: &mut Budget,
    ) -> Result<Self, OptimizationError> {
        budget.charge(shape.functions)?;
        let mut functions = Vec::with_capacity(program.functions.len());
        for function in &program.functions {
            let value_count = function.blocks.iter().try_fold(0_usize, |sum, block| {
                sum.checked_add(block.parameters.len())
                    .and_then(|value| value.checked_add(block.instructions.len()))
                    .ok_or_else(budget_error)
            })?;
            budget.charge((value_count as u64).saturating_mul(2))?;
            let mut definitions = vec![None; value_count];
            let mut constants = vec![None; value_count];
            for block in &function.blocks {
                for parameter in &block.parameters {
                    checker_store_definition(
                        &mut definitions,
                        parameter.id,
                        CheckerDefinition {
                            block: block.id,
                            instruction_index: None,
                            ty: &parameter.ty,
                            instruction: None,
                        },
                    )?;
                }
                for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                    checker_store_definition(
                        &mut definitions,
                        instruction.id,
                        CheckerDefinition {
                            block: block.id,
                            instruction_index: Some(instruction_index),
                            ty: &instruction.ty,
                            instruction: Some(instruction),
                        },
                    )?;
                    if let InstructionKind::Constant(Constant::I64(value)) = instruction.kind {
                        let slot = instruction
                            .id
                            .index()
                            .and_then(|index| constants.get_mut(index))
                            .ok_or_else(|| {
                                input_index_error("checker constant ValueId is not dense")
                            })?;
                        *slot = Some(value);
                    }
                    budget.charge(1)?;
                }
            }
            let dominance = CheckerDominance::derive(function, budget)?;
            let mut function_indexes = CheckerFunctionIndexes {
                definitions,
                constants,
                expressions: HashMap::with_capacity(value_count),
                dominance,
            };
            for block in &function.blocks {
                if !function_indexes.dominance.reached(block.id) {
                    continue;
                }
                for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                    if let Some(key) =
                        checker_exact_expression_key(&function_indexes, instruction, budget)?
                    {
                        budget.charge(1)?;
                        function_indexes.expressions.entry(key).or_default().push(
                            CheckerPosition {
                                block: block.id,
                                instruction_index,
                                value: instruction.id,
                            },
                        );
                    }
                }
            }
            functions.push(function_indexes);
        }
        Ok(Self { functions })
    }

    fn verify_records(
        &self,
        program: &Program,
        certificate: &OptimizationCertificate,
        budget: &mut Budget,
    ) -> Result<(), OptimizationError> {
        let mut sequence = 0_usize;
        for function in &program.functions {
            let function_index = function
                .id
                .index()
                .ok_or_else(|| input_index_error("checker FunctionId cannot index input"))?;
            let indexes = self
                .functions
                .get(function_index)
                .ok_or_else(|| input_index_error("checker function index is incomplete"))?;
            for block in &function.blocks {
                if !indexes.dominance.reached(block.id) {
                    continue;
                }
                for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                    budget.charge(1)?;
                    let identity = checker_allowed_identity(
                        indexes,
                        block.id,
                        instruction_index,
                        instruction,
                        budget,
                    )?;
                    let expected = if identity.is_some() {
                        identity
                    } else {
                        checker_allowed_gvn(
                            indexes,
                            block.id,
                            instruction_index,
                            instruction,
                            budget,
                        )?
                    };
                    if let Some((kind, operation, operands, replacement)) = expected {
                        let record = certificate.records.get(sequence).ok_or_else(|| {
                            certificate_mismatch("certificate is missing a canonical edit")
                        })?;
                        budget.charge(1_u64.saturating_add(operands.len() as u64))?;
                        if record.sequence != sequence as u64
                            || record.function != function.id
                            || record.block != block.id
                            || record.value != instruction.id
                            || record.kind != kind
                            || record.expected_operation != operation
                            || record.expected_operands != operands
                            || record.replacement != replacement
                        {
                            return Err(certificate_mismatch(
                                "certificate record does not match independently derived edit",
                            ));
                        }
                        sequence = sequence.saturating_add(1);
                    }
                }
            }
        }
        if sequence != certificate.records.len() {
            return Err(certificate_mismatch(
                "certificate has an extra, unreachable, stale, or reordered edit",
            ));
        }
        Ok(())
    }
}

fn checker_store_definition<'a>(
    definitions: &mut [Option<CheckerDefinition<'a>>],
    value: ValueId,
    definition: CheckerDefinition<'a>,
) -> Result<(), OptimizationError> {
    let index = value
        .index()
        .ok_or_else(|| input_index_error("checker ValueId cannot index"))?;
    let slot = definitions
        .get_mut(index)
        .ok_or_else(|| input_index_error("checker ValueId is outside dense index"))?;
    if slot.replace(definition).is_some() {
        return Err(input_index_error("checker found duplicate ValueId"));
    }
    Ok(())
}

fn certificate_mismatch(detail: &str) -> OptimizationError {
    OptimizationError::new(OptimizationFailureCode::CertificateMismatch, detail)
}

fn checker_allowed_identity(
    indexes: &CheckerFunctionIndexes<'_>,
    use_block: BlockId,
    use_index: usize,
    instruction: &Instruction,
    budget: &mut Budget,
) -> Result<Option<LegalEdit>, OptimizationError> {
    let InstructionKind::Runtime {
        operation,
        arguments,
        signature,
    } = &instruction.kind
    else {
        return Ok(None);
    };
    budget.charge(arguments.len() as u64)?;
    let metadata_legal = instruction.metadata.effects == EffectSet::PURE
        && instruction.metadata.failure == FailureBehavior::None
        && instruction.metadata.safepoint == Safepoint::None
        && instruction.metadata.frame_state.is_none();
    if !metadata_legal
        || !checker_signature_and_operand_types(indexes, signature, arguments, &instruction.ty)
    {
        return Ok(None);
    }
    let replacement = match (*operation, arguments.as_slice(), &instruction.ty) {
        (RuntimeOp::BitXor | RuntimeOp::BitOr, [left, right], SsaType::I64) => {
            if checker_i64_constant(indexes, *left) == Some(0) {
                Some(*right)
            } else if checker_i64_constant(indexes, *right) == Some(0)
                || *operation == RuntimeOp::BitOr && left == right
            {
                Some(*left)
            } else {
                None
            }
        }
        (RuntimeOp::BitAnd, [left, right], SsaType::I64) => {
            if checker_i64_constant(indexes, *left) == Some(-1) {
                Some(*right)
            } else if checker_i64_constant(indexes, *right) == Some(-1) || left == right {
                Some(*left)
            } else {
                None
            }
        }
        (RuntimeOp::Not, [inner], SsaType::Bool) => {
            let Some(inner_definition) = indexes
                .definitions
                .get(inner.index().unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
            else {
                return Ok(None);
            };
            let Some(inner_instruction) = inner_definition.instruction else {
                return Ok(None);
            };
            if inner_instruction.ty != SsaType::Bool
                || inner_instruction.metadata.effects != EffectSet::PURE
                || inner_instruction.metadata.failure != FailureBehavior::None
                || inner_instruction.metadata.safepoint != Safepoint::None
                || inner_instruction.metadata.frame_state.is_some()
            {
                None
            } else {
                match &inner_instruction.kind {
                    InstructionKind::Runtime {
                        operation: RuntimeOp::Not,
                        arguments: inner_arguments,
                        signature: inner_signature,
                    } if inner_arguments.len() == 1
                        && checker_signature_and_operand_types(
                            indexes,
                            inner_signature,
                            inner_arguments,
                            &inner_instruction.ty,
                        ) =>
                    {
                        inner_arguments.first().copied()
                    }
                    _ => None,
                }
            }
        }
        _ => None,
    };
    let Some(replacement) = replacement else {
        return Ok(None);
    };
    if !checker_value_precedes(indexes, replacement, use_block, use_index) {
        return Ok(None);
    }
    Ok(Some((
        OptimizationEditKind::AlgebraicIdentity,
        *operation,
        arguments.clone(),
        replacement,
    )))
}

fn checker_value_precedes(
    indexes: &CheckerFunctionIndexes<'_>,
    value: ValueId,
    use_block: BlockId,
    use_index: usize,
) -> bool {
    let Some(definition) = indexes
        .definitions
        .get(value.index().unwrap_or(usize::MAX))
        .and_then(Option::as_ref)
    else {
        return false;
    };
    if definition.block == use_block {
        return definition
            .instruction_index
            .is_none_or(|definition_index| definition_index < use_index);
    }
    indexes.dominance.proves_definition_before(
        definition.block,
        definition.instruction_index.unwrap_or(0),
        use_block,
        use_index,
    )
}

fn checker_allowed_gvn(
    indexes: &CheckerFunctionIndexes<'_>,
    use_block: BlockId,
    use_index: usize,
    instruction: &Instruction,
    budget: &mut Budget,
) -> Result<Option<LegalEdit>, OptimizationError> {
    let Some(key) = checker_exact_expression_key(indexes, instruction, budget)? else {
        return Ok(None);
    };
    let Some(candidates) = indexes.expressions.get(&key) else {
        return Ok(None);
    };
    let mut best: Option<CheckerPosition> = None;
    for candidate in candidates {
        budget.charge(1)?;
        if candidate.value == instruction.id
            || !indexes.dominance.proves_definition_before(
                candidate.block,
                candidate.instruction_index,
                use_block,
                use_index,
            )
        {
            continue;
        }
        let candidate_order = (
            candidate.block.raw(),
            candidate.instruction_index,
            candidate.value.raw(),
        );
        if best.is_none_or(|old| {
            candidate_order < (old.block.raw(), old.instruction_index, old.value.raw())
        }) {
            best = Some(*candidate);
        }
    }
    let Some(best) = best else {
        return Ok(None);
    };
    let InstructionKind::Runtime {
        operation,
        arguments,
        ..
    } = &instruction.kind
    else {
        return Ok(None);
    };
    let exact_checked_i64 = instruction.ty == SsaType::I64
        && matches!(
            operation,
            RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide
        )
        && arguments.iter().all(|argument| {
            indexes
                .definitions
                .get(argument.index().unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .is_some_and(|definition| definition.ty == &SsaType::I64)
        })
        && instruction.metadata.effects == EffectSet::MAY_TRAP
        && instruction.metadata.failure == FailureBehavior::Trap;
    let kind = if exact_checked_i64 {
        OptimizationEditKind::CheckedI64GlobalValueNumbering
    } else {
        OptimizationEditKind::GlobalValueNumbering
    };
    Ok(Some((kind, *operation, arguments.clone(), best.value)))
}

fn checker_exact_expression_key<'a>(
    indexes: &CheckerFunctionIndexes<'a>,
    instruction: &'a Instruction,
    budget: &mut Budget,
) -> Result<Option<CheckerExpressionKey<'a>>, OptimizationError> {
    let InstructionKind::Runtime {
        operation,
        arguments,
        signature,
    } = &instruction.kind
    else {
        return Ok(None);
    };
    budget.charge(arguments.len() as u64)?;
    if instruction.metadata.safepoint != Safepoint::None
        || instruction.metadata.frame_state.is_some()
        || !checker_is_nonownership_scalar(&instruction.ty)
        || !checker_signature_and_operand_types(indexes, signature, arguments, &instruction.ty)
    {
        return Ok(None);
    }
    let exact_pure = match operation {
        RuntimeOp::Less | RuntimeOp::LessEqual | RuntimeOp::Greater | RuntimeOp::GreaterEqual => {
            instruction.ty == SsaType::Bool
                && arguments.len() == 2
                && arguments.iter().all(|argument| {
                    indexes
                        .definitions
                        .get(argument.index().unwrap_or(usize::MAX))
                        .and_then(Option::as_ref)
                        .is_some_and(|definition| {
                            matches!(definition.ty, SsaType::I64 | SsaType::F64)
                        })
                })
        }
        RuntimeOp::Not => instruction.ty == SsaType::Bool && arguments.len() == 1,
        RuntimeOp::BitAnd | RuntimeOp::BitOr | RuntimeOp::BitXor => {
            instruction.ty == SsaType::I64
                && arguments.len() == 2
                && arguments.iter().all(|argument| {
                    indexes
                        .definitions
                        .get(argument.index().unwrap_or(usize::MAX))
                        .and_then(Option::as_ref)
                        .is_some_and(|definition| definition.ty == &SsaType::I64)
                })
        }
        _ => false,
    } && instruction.metadata.effects == EffectSet::PURE
        && instruction.metadata.failure == FailureBehavior::None;
    let exact_arithmetic = matches!(
        operation,
        RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide
    ) && arguments.len() == 2
        && arguments.iter().all(|argument| {
            indexes
                .definitions
                .get(argument.index().unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .is_some_and(|definition| matches!(definition.ty, SsaType::I64 | SsaType::F64))
        })
        && matches!(instruction.ty, SsaType::I64 | SsaType::F64)
        && instruction.metadata.effects == EffectSet::MAY_TRAP
        && instruction.metadata.failure == FailureBehavior::Trap;
    if !exact_pure && !exact_arithmetic {
        return Ok(None);
    }
    Ok(Some(CheckerExpressionKey {
        operation: *operation,
        arguments,
        signature,
        result_type: &instruction.ty,
    }))
}

fn checker_signature_and_operand_types(
    indexes: &CheckerFunctionIndexes<'_>,
    signature: &Signature,
    arguments: &[ValueId],
    result: &SsaType,
) -> bool {
    if !signature.type_parameters.is_empty()
        || !signature.bounds.is_empty()
        || signature.result.as_ref() != result
        || signature.parameters.len() != arguments.len()
    {
        return false;
    }
    signature
        .parameters
        .iter()
        .zip(arguments)
        .all(|(expected, argument)| {
            checker_is_nonownership_scalar(expected)
                && indexes
                    .definitions
                    .get(argument.index().unwrap_or(usize::MAX))
                    .and_then(Option::as_ref)
                    .is_some_and(|definition| definition.ty == expected)
        })
}

fn checker_i64_constant(indexes: &CheckerFunctionIndexes<'_>, value: ValueId) -> Option<i64> {
    indexes.constants.get(value.index()?)?.as_ref().copied()
}

fn checker_is_nonownership_scalar(ty: &SsaType) -> bool {
    matches!(
        ty,
        SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
    )
}

struct CheckerDominance {
    matrix: Vec<Vec<u64>>,
    reachable: Vec<bool>,
}

impl CheckerDominance {
    fn derive(function: &Function, budget: &mut Budget) -> Result<Self, OptimizationError> {
        let block_count = function.blocks.len();
        let word_count = block_count.saturating_add(63) / 64;
        budget.charge(
            (block_count as u64)
                .saturating_mul(word_count as u64)
                .saturating_add((block_count as u64).saturating_mul(3)),
        )?;
        let mut incoming = vec![Vec::<usize>::new(); block_count];
        for source_block in &function.blocks {
            let source_index = source_block
                .id
                .index()
                .ok_or_else(|| input_index_error("checker source block ID cannot index"))?;
            match &source_block.terminator {
                crate::Terminator::Branch { target, .. } => {
                    checker_add_incoming(&mut incoming, *target, source_index, budget)?;
                }
                crate::Terminator::ConditionalBranch {
                    true_target,
                    false_target,
                    ..
                } => {
                    checker_add_incoming(&mut incoming, *true_target, source_index, budget)?;
                    checker_add_incoming(&mut incoming, *false_target, source_index, budget)?;
                }
                _ => {}
            }
        }
        let entry = function
            .entry
            .index()
            .ok_or_else(|| input_index_error("checker entry block ID cannot index"))?;
        let mut reachable = vec![false; block_count];
        let mut worklist = VecDeque::new();
        let entry_slot = reachable
            .get_mut(entry)
            .ok_or_else(|| input_index_error("checker entry block is outside function"))?;
        *entry_slot = true;
        worklist.push_back(entry);
        while let Some(source) = worklist.pop_front() {
            budget.charge(1)?;
            let terminator = &function
                .blocks
                .get(source)
                .ok_or_else(|| input_index_error("checker reachability source is missing"))?
                .terminator;
            match terminator {
                crate::Terminator::Branch { target, .. } => {
                    checker_mark_reachable(&mut reachable, &mut worklist, *target)?;
                }
                crate::Terminator::ConditionalBranch {
                    true_target,
                    false_target,
                    ..
                } => {
                    checker_mark_reachable(&mut reachable, &mut worklist, *true_target)?;
                    checker_mark_reachable(&mut reachable, &mut worklist, *false_target)?;
                }
                _ => {}
            }
        }
        let mut matrix = vec![vec![0_u64; word_count]; block_count];
        for row in 0..block_count {
            if reachable[row] {
                for candidate in 0..block_count {
                    if reachable[candidate] {
                        matrix[row][candidate / 64] |= 1_u64 << (candidate % 64);
                        budget.charge(1)?;
                    }
                }
            }
            matrix[row][row / 64] |= 1_u64 << (row % 64);
        }
        matrix[entry].fill(0);
        matrix[entry][entry / 64] |= 1_u64 << (entry % 64);
        let mut changed = true;
        while changed {
            changed = false;
            for row in 0..block_count {
                if row == entry || !reachable[row] {
                    continue;
                }
                budget.charge(word_count as u64)?;
                let mut intersection = vec![u64::MAX; word_count];
                let mut any = false;
                for predecessor in incoming[row]
                    .iter()
                    .copied()
                    .filter(|predecessor| reachable[*predecessor])
                {
                    any = true;
                    for word_index in 0..word_count {
                        intersection[word_index] &= matrix[predecessor][word_index];
                        budget.charge(1)?;
                    }
                }
                if !any {
                    intersection.fill(0);
                }
                if let Some(last) = intersection.last_mut() {
                    let unused = word_count.saturating_mul(64).saturating_sub(block_count);
                    if unused != 0 {
                        *last &= u64::MAX >> unused;
                    }
                }
                intersection[row / 64] |= 1_u64 << (row % 64);
                if matrix[row] != intersection {
                    matrix[row] = intersection;
                    changed = true;
                }
            }
        }
        Ok(Self { matrix, reachable })
    }

    fn reached(&self, block: BlockId) -> bool {
        block
            .index()
            .and_then(|index| self.reachable.get(index))
            .copied()
            .unwrap_or(false)
    }

    fn proves_definition_before(
        &self,
        definition_block: BlockId,
        definition_instruction: usize,
        use_block: BlockId,
        use_instruction: usize,
    ) -> bool {
        if !self.reached(definition_block) || !self.reached(use_block) {
            return false;
        }
        if definition_block == use_block {
            return definition_instruction < use_instruction;
        }
        let Some(definition_index) = definition_block.index() else {
            return false;
        };
        use_block
            .index()
            .and_then(|row| self.matrix.get(row))
            .and_then(|bits| bits.get(definition_index / 64))
            .is_some_and(|word| word & (1_u64 << (definition_index % 64)) != 0)
    }
}

fn checker_add_incoming(
    incoming: &mut [Vec<usize>],
    target: BlockId,
    source: usize,
    budget: &mut Budget,
) -> Result<(), OptimizationError> {
    let target = target
        .index()
        .ok_or_else(|| input_index_error("checker successor block ID cannot index"))?;
    let list = incoming
        .get_mut(target)
        .ok_or_else(|| input_index_error("checker successor block is outside function"))?;
    budget.charge(1)?;
    list.push(source);
    Ok(())
}

fn checker_mark_reachable(
    reachable: &mut [bool],
    worklist: &mut VecDeque<usize>,
    block: BlockId,
) -> Result<(), OptimizationError> {
    let index = block
        .index()
        .ok_or_else(|| input_index_error("checker reachable block ID cannot index"))?;
    let slot = reachable
        .get_mut(index)
        .ok_or_else(|| input_index_error("checker reachable block is outside function"))?;
    if !*slot {
        *slot = true;
        worklist.push_back(index);
    }
    Ok(())
}

fn discovery_reconstruct(
    input: &Program,
    certificate: &OptimizationCertificate,
    indexes: &DiscoveryIndexes<'_>,
    budget: &mut Budget,
) -> Result<Program, OptimizationError> {
    budget.reconstruction_passes = budget.reconstruction_passes.saturating_add(1);
    let shape = preflight_program(input, budget)?;
    budget.charge(shape.allocation_units())?;
    let mut candidate = input.clone();
    for (sequence, record) in certificate.records.iter().enumerate() {
        if record.sequence != sequence as u64 {
            return Err(illegal_edit(
                "certificate sequence is not dense and ordered",
            ));
        }
        let function_index = record
            .function
            .index()
            .ok_or_else(|| illegal_edit("certificate function ID cannot index"))?;
        let function_indexes = indexes
            .functions
            .get(function_index)
            .ok_or_else(|| illegal_edit("certificate function ID is stale"))?;
        let definition = function_indexes
            .definitions
            .get(record.value.index().unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .ok_or_else(|| illegal_edit("certificate value ID is stale"))?;
        discovery_apply_record(
            &mut candidate,
            record,
            definition.block,
            definition.instruction_index,
            budget,
        )?;
    }
    budget.check_growth(instruction_count(&candidate))?;
    run_cleanup(candidate, budget)
}

fn checker_reconstruct(
    input: &Program,
    certificate: &OptimizationCertificate,
    indexes: &CheckerIndexes<'_>,
    budget: &mut Budget,
) -> Result<Program, OptimizationError> {
    budget.reconstruction_passes = budget.reconstruction_passes.saturating_add(1);
    let shape = preflight_program(input, budget)?;
    budget.charge(shape.allocation_units())?;
    let mut private = input.clone();
    for (sequence, record) in certificate.records.iter().enumerate() {
        budget.charge(1_u64.saturating_add(record.expected_operands.len() as u64))?;
        if record.sequence != sequence as u64 {
            return Err(illegal_edit("checker record sequence is not dense"));
        }
        let function_index = record
            .function
            .index()
            .ok_or_else(|| illegal_edit("checker record function cannot index"))?;
        let function_indexes = indexes
            .functions
            .get(function_index)
            .ok_or_else(|| illegal_edit("checker record function is stale"))?;
        let definition = function_indexes
            .definitions
            .get(record.value.index().unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .ok_or_else(|| illegal_edit("checker record value is stale"))?;
        let instruction = definition
            .instruction
            .ok_or_else(|| illegal_edit("checker record targets a block parameter"))?;
        match &instruction.kind {
            InstructionKind::Runtime {
                operation,
                arguments,
                ..
            } if *operation == record.expected_operation
                && *arguments == record.expected_operands => {}
            _ => {
                return Err(illegal_edit(
                    "checker record operation or operands differ from immutable input",
                ));
            }
        }
        checker_apply_record(
            &mut private,
            record,
            definition.block,
            definition.instruction_index,
        )?;
    }
    budget.check_growth(instruction_count(&private))?;
    run_cleanup(private, budget)
}

fn illegal_edit(detail: &str) -> OptimizationError {
    OptimizationError::new(OptimizationFailureCode::IllegalEdit, detail)
}

fn discovery_apply_record(
    candidate: &mut Program,
    record: &OptimizationCertificateRecord,
    definition_block: BlockId,
    definition_index: Option<usize>,
    budget: &mut Budget,
) -> Result<(), OptimizationError> {
    budget.charge(1_u64.saturating_add(record.expected_operands.len() as u64))?;
    if definition_block != record.block {
        return Err(illegal_edit("certificate block ID is stale"));
    }
    let instruction_index =
        definition_index.ok_or_else(|| illegal_edit("certificate targets a block parameter"))?;
    let function = candidate
        .functions
        .get_mut(record.function.index().unwrap_or(usize::MAX))
        .filter(|function| function.id == record.function)
        .ok_or_else(|| illegal_edit("certificate function ID is stale"))?;
    let block = function
        .blocks
        .get_mut(record.block.index().unwrap_or(usize::MAX))
        .filter(|block| block.id == record.block)
        .ok_or_else(|| illegal_edit("certificate block ID is stale"))?;
    let instruction = block
        .instructions
        .get_mut(instruction_index)
        .filter(|instruction| instruction.id == record.value)
        .ok_or_else(|| illegal_edit("certificate value ID is stale"))?;
    match &instruction.kind {
        InstructionKind::Runtime {
            operation,
            arguments,
            ..
        } if *operation == record.expected_operation && *arguments == record.expected_operands => {}
        _ => return Err(illegal_edit("certificate operation or operands are stale")),
    }
    replace_with_copy(instruction, record.replacement);
    Ok(())
}

fn checker_apply_record(
    candidate: &mut Program,
    record: &OptimizationCertificateRecord,
    definition_block: BlockId,
    definition_index: Option<usize>,
) -> Result<(), OptimizationError> {
    if definition_block != record.block {
        return Err(illegal_edit(
            "checker record block differs from private index",
        ));
    }
    let instruction_index = definition_index
        .ok_or_else(|| illegal_edit("checker record has no instruction location"))?;
    let function = candidate
        .functions
        .get_mut(record.function.index().unwrap_or(usize::MAX))
        .filter(|function| function.id == record.function)
        .ok_or_else(|| illegal_edit("checker private function is stale"))?;
    let block = function
        .blocks
        .get_mut(record.block.index().unwrap_or(usize::MAX))
        .filter(|block| block.id == record.block)
        .ok_or_else(|| illegal_edit("checker private block is stale"))?;
    let instruction = block
        .instructions
        .get_mut(instruction_index)
        .filter(|instruction| instruction.id == record.value)
        .ok_or_else(|| illegal_edit("checker private instruction is stale"))?;
    match &instruction.kind {
        InstructionKind::Runtime {
            operation,
            arguments,
            ..
        } if *operation == record.expected_operation && *arguments == record.expected_operands => {}
        _ => {
            return Err(illegal_edit(
                "checker private operation or operands do not match record",
            ));
        }
    }
    replace_with_copy(instruction, record.replacement);
    Ok(())
}

fn replace_with_copy(instruction: &mut Instruction, replacement: ValueId) {
    instruction.kind = InstructionKind::Copy(replacement);
    instruction.metadata.effects = EffectSet::PURE;
    instruction.metadata.safepoint = Safepoint::None;
    instruction.metadata.failure = FailureBehavior::None;
    instruction.metadata.frame_state = None;
}

fn run_cleanup(candidate: Program, budget: &mut Budget) -> Result<Program, OptimizationError> {
    let shape = preflight_program(&candidate, budget)?;
    budget.charge_validation(&shape)?;
    let mut verified = verify(candidate)
        .map_err(|error| illegal_edit(&format!("certified edit-stage SSA failed: {error}")))?;
    // Delete disconnected components before copy propagation so legal
    // unreachable SCCs cannot retain cross-block copy references while dense
    // values are compacted.
    verified = cleanup_pass(verified, unreachable_blocks, budget)?;
    verified = cleanup_pass(verified, copy_propagate, budget)?;
    verified = cleanup_pass(verified, simplify_branches, budget)?;
    verified = cleanup_pass(verified, empty_block_forwarding, budget)?;
    verified = cleanup_pass(verified, effect_aware_dce, budget)?;
    verified = cleanup_pass(verified, direct_call_resolution, budget)?;
    verified = cleanup_pass(verified, canonical_block_order, budget)?;
    Ok(verified.into_program())
}

fn cleanup_pass(
    input: VerifiedProgram,
    pass: fn(&VerifiedProgram) -> crate::Result<VerifiedProgram>,
    budget: &mut Budget,
) -> Result<VerifiedProgram, OptimizationError> {
    budget.charge_cleanup_pass()?;
    let shape = preflight_program(input.program(), budget)?;
    let worst_case_pass_work = shape
        .allocation_units()
        .saturating_add(shape.blocks.saturating_mul(shape.blocks))
        .saturating_add(shape.instructions)
        .saturating_add(shape.operands);
    budget.charge(worst_case_pass_work)?;
    // Each isolated baseline pass ordinarily verifies the output once.
    budget.charge_validation(&shape)?;
    let output = pass(&input).map_err(|error| {
        OptimizationError::new(
            OptimizationFailureCode::OutputVerification,
            error.to_string(),
        )
    })?;
    budget.check_growth(instruction_count(output.program()))?;
    Ok(output)
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

fn exact_program_equal(left: &Program, right: &Program) -> bool {
    left.sources == right.sources
        && left.products == right.products
        && left.traits == right.traits
        && left.implementations == right.implementations
        && left.main == right.main
        && left.functions.len() == right.functions.len()
        && left
            .functions
            .iter()
            .zip(&right.functions)
            .all(|(left, right)| exact_function_equal(left, right))
}

fn exact_function_equal(left: &Function, right: &Function) -> bool {
    left.id == right.id
        && left.name == right.name
        && left.signature == right.signature
        && left.places == right.places
        && left.effects == right.effects
        && left.entry == right.entry
        && left.origin == right.origin
        && left.blocks.len() == right.blocks.len()
        && left.blocks.iter().zip(&right.blocks).all(|(left, right)| {
            left.id == right.id
                && left.parameters == right.parameters
                && left.terminator == right.terminator
                && left.metadata == right.metadata
                && left.instructions.len() == right.instructions.len()
                && left
                    .instructions
                    .iter()
                    .zip(&right.instructions)
                    .all(|(left, right)| exact_instruction_equal(left, right))
        })
}

fn exact_instruction_equal(left: &Instruction, right: &Instruction) -> bool {
    left.id == right.id
        && left.ty == right.ty
        && left.metadata == right.metadata
        && exact_instruction_kind_equal(&left.kind, &right.kind)
}

fn exact_instruction_kind_equal(left: &InstructionKind, right: &InstructionKind) -> bool {
    match (left, right) {
        (InstructionKind::Constant(left), InstructionKind::Constant(right)) => {
            exact_constant_equal(left, right)
        }
        (InstructionKind::Copy(left), InstructionKind::Copy(right)) => left == right,
        (
            InstructionKind::PlaceInit {
                place: left_place,
                value: left_value,
            },
            InstructionKind::PlaceInit {
                place: right_place,
                value: right_value,
            },
        ) => left_place == right_place && left_value == right_value,
        (InstructionKind::PlaceEnd { place: left }, InstructionKind::PlaceEnd { place: right }) => {
            left == right
        }
        (
            InstructionKind::Move {
                place: left_place,
                value: left_value,
            },
            InstructionKind::Move {
                place: right_place,
                value: right_value,
            },
        ) => left_place == right_place && left_value == right_value,
        (
            InstructionKind::Borrow {
                place: left_place,
                loan: left_loan,
                kind: left_kind,
                value: left_value,
            },
            InstructionKind::Borrow {
                place: right_place,
                loan: right_loan,
                kind: right_kind,
                value: right_value,
            },
        ) => {
            left_place == right_place
                && left_loan == right_loan
                && left_kind == right_kind
                && left_value == right_value
        }
        (InstructionKind::FunctionRef(left), InstructionKind::FunctionRef(right)) => left == right,
        (
            InstructionKind::Runtime {
                operation: left_operation,
                arguments: left_arguments,
                signature: left_signature,
            },
            InstructionKind::Runtime {
                operation: right_operation,
                arguments: right_arguments,
                signature: right_signature,
            },
        ) => {
            left_operation == right_operation
                && left_arguments == right_arguments
                && left_signature == right_signature
        }
        (
            InstructionKind::Call {
                target: left_target,
                arguments: left_arguments,
                signature: left_signature,
                instantiation: left_instantiation,
            },
            InstructionKind::Call {
                target: right_target,
                arguments: right_arguments,
                signature: right_signature,
                instantiation: right_instantiation,
            },
        ) => {
            left_target == right_target
                && left_arguments == right_arguments
                && left_signature == right_signature
                && left_instantiation == right_instantiation
        }
        (
            InstructionKind::ProductValue {
                product: left_product,
                fields: left_fields,
            },
            InstructionKind::ProductValue {
                product: right_product,
                fields: right_fields,
            },
        ) => left_product == right_product && left_fields == right_fields,
        (
            InstructionKind::ProductField {
                product: left_product,
                field: left_field,
                value: left_value,
            },
            InstructionKind::ProductField {
                product: right_product,
                field: right_field,
                value: right_value,
            },
        ) => {
            left_product == right_product && left_field == right_field && left_value == right_value
        }
        (
            InstructionKind::WithProductField {
                product: left_product,
                field: left_field,
                value: left_value,
                replacement: left_replacement,
            },
            InstructionKind::WithProductField {
                product: right_product,
                field: right_field,
                value: right_value,
                replacement: right_replacement,
            },
        ) => {
            left_product == right_product
                && left_field == right_field
                && left_value == right_value
                && left_replacement == right_replacement
        }
        _ => false,
    }
}

fn exact_constant_equal(left: &Constant, right: &Constant) -> bool {
    match (left, right) {
        (Constant::F64(left), Constant::F64(right)) => left.to_bits() == right.to_bits(),
        (Constant::Unit, Constant::Unit)
        | (Constant::EmptyList, Constant::EmptyList)
        | (Constant::None, Constant::None) => true,
        (Constant::Bool(left), Constant::Bool(right)) => left == right,
        (Constant::I64(left), Constant::I64(right)) => left == right,
        (Constant::Str(left), Constant::Str(right))
        | (Constant::Symbol(left), Constant::Symbol(right)) => left == right,
        _ => false,
    }
}
