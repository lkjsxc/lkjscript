use super::*;
use std::cell::Cell;

#[path = "demanded_callable_read_fixture.rs"]
mod fixture;
use fixture::Fixture;

thread_local! {
    static IN_PROOF: Cell<bool> = const { Cell::new(false) };
}

pub(super) struct Phase(bool);
impl Phase {
    pub(super) fn enter() -> Self {
        Self(IN_PROOF.with(|active| active.replace(true)))
    }
}
impl Drop for Phase {
    fn drop(&mut self) {
        IN_PROOF.with(|active| active.set(self.0));
    }
}

const WORK_LIMIT: usize = 10_000_000;

#[test]
fn incoming_mapping_is_read_once_across_all_demanded_witness_paths() -> Result<(), Diagnostic> {
    let mut counts = Vec::new();
    for width in [1, 4, 16, 64] {
        let fixture = Fixture::new(width, None)?;
        let mut work = 0;
        let observation = validate_callable_closure(&fixture, &mut work, WORK_LIMIT)?;
        let reads = fixture.mapping_reads.get();
        let lexical_reads = fixture.lexical_reads.get();
        let type_reads = fixture.type_reads.get();
        eprintln!(
            "incoming-input width={width} mapping_reads={reads} lexical_reads={lexical_reads} type_reads={type_reads} work={work} observation={observation:?}"
        );
        assert!(observation.slots >= width + 2);
        assert_eq!(observation.edges, width * 2 + 1);
        counts.push((width, reads, lexical_reads, type_reads));
    }
    // Check after the entire ladder so a predecessor retains every adverse point.
    for (width, reads, lexical_reads, type_reads) in counts {
        assert_eq!(reads, 1, "one mapping was reread for {width} witness paths");
        assert_eq!(
            lexical_reads,
            width + 1,
            "lexical declaration rereads at width {width}"
        );
        assert_eq!(
            type_reads,
            width + 1,
            "type flow still visits every demanded input"
        );
    }
    Ok(())
}

#[test]
fn undemanded_acyclic_mapping_keeps_input_projection_lazy() -> Result<(), Diagnostic> {
    let fixture = Fixture::new(0, None)?;
    let observation = validate_callable_closure(&fixture, &mut 0, WORK_LIMIT)?;
    assert_eq!(fixture.mapping_reads.get(), 0);
    assert_eq!(fixture.lexical_reads.get(), 0);
    assert_eq!(fixture.type_reads.get(), 0);
    assert_eq!(observation.edges, 0);
    Ok(())
}

#[test]
fn shared_inputs_keep_growth_at_every_distinct_witness_position() -> Result<(), Diagnostic> {
    for width in [1, 4, 16] {
        for index in 0..width {
            let fixture = Fixture::new(width, Some(index))?;
            let result = validate_callable_closure(&fixture, &mut 0, WORK_LIMIT);
            let error = match result {
                Err(error) => error,
                Ok(_) => {
                    return Err(semantic(
                        "input_test_missing_growth",
                        "constructed witness growth was accepted",
                    ));
                }
            };
            assert_eq!(error.class, DiagnosticClass::Semantic);
            assert!(
                error.code.contains("expan"),
                "unexpected rejection: {error:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn shared_input_hits_keep_exact_work_and_late_cancellation_boundaries() -> Result<(), Diagnostic> {
    let fixture = Fixture::new(16, None)?;
    let mut work = 0;
    let expected = validate_callable_closure(&fixture, &mut work, WORK_LIMIT)?;
    let checkpoints = fixture.proof_checkpoints.get();
    assert!(checkpoints > 8);
    fixture.reset(None);
    assert_eq!(validate_callable_closure(&fixture, &mut 0, work)?, expected);
    fixture.reset(None);
    let refused = validate_callable_closure(&fixture, &mut 0, work - 1);
    assert!(matches!(
        refused,
        Err(Diagnostic {
            class: DiagnosticClass::Resource,
            ..
        })
    ));
    for cutoff in [1, 3, checkpoints / 2, checkpoints - 1] {
        fixture.reset(Some(cutoff));
        let cancelled = validate_callable_closure(&fixture, &mut 0, WORK_LIMIT);
        assert!(
            matches!(cancelled, Err(Diagnostic { class: DiagnosticClass::Cancelled, ref code, .. }) if code == "input_proof_cancelled")
        );
        fixture.reset(None);
        assert_eq!(
            validate_callable_closure(&fixture, &mut 0, WORK_LIMIT)?,
            expected
        );
        assert_eq!(fixture.mapping_reads.get(), 1);
    }
    Ok(())
}
