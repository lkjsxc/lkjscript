use crate::optimize::*;
use crate::{verify, Program, VerifiedProgram};

/// Discover deterministic edits, build a private candidate, and submit both to
/// the independent certificate boundary under one aggregate budget.
pub fn optimize(
    input: &VerifiedProgram,
    limits: OptimizationLimits,
) -> Result<VerifiedOptimizedProgram, OptimizationError> {
    let mut budget = Budget::new(limits);
    let input_shape = preflight_program(input.program(), &mut budget)?;
    budget.set_input_instructions(input_shape.instructions);
    if has_explicit_structural_operations(input.program()) {
        let certificate = OptimizationCertificate {
            records: Vec::new(),
        };
        let checker = CheckerIndexes::build(input.program(), &input_shape, &mut budget)?;
        let candidate = checker_reconstruct(input.program(), &certificate, &checker, &mut budget)?;
        let candidate_shape = preflight_program(&candidate, &mut budget)?;
        return verify_optimization_with_budget(
            input,
            candidate,
            candidate_shape,
            certificate,
            input_shape,
            &mut budget,
        );
    }

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

fn has_explicit_structural_operations(program: &Program) -> bool {
    program.functions.iter().any(|function| {
        function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(
                    instruction.kind,
                    crate::InstructionKind::StructuralPublish { .. }
                        | crate::InstructionKind::DestinationCreate { .. }
                        | crate::InstructionKind::DestinationFieldInit { .. }
                        | crate::InstructionKind::DestinationFinish { .. }
                        | crate::InstructionKind::DestinationAbort { .. }
                        | crate::InstructionKind::AggregateFieldBorrow { .. }
                        | crate::InstructionKind::AggregateTag { .. }
                        | crate::InstructionKind::AggregateConsumePayload { .. }
                        | crate::InstructionKind::StringUtf8View { .. }
                        | crate::InstructionKind::StructuralCopy { .. }
                )
            })
        })
    })
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

pub(crate) fn verify_optimization_with_budget(
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
    if !(certificate.records.is_empty() && has_explicit_structural_operations(input.program())) {
        checker.verify_records(input.program(), &certificate, budget)?;
    }
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
