//! Compare exactly the replaced lookup, not complete validation or runtime speed.
use super::*;
use std::time::Instant;

struct Observation {
    reads: usize,
    work: usize,
    metadata: usize,
    elapsed_ns: u128,
}

fn measure(
    source: &Source,
    owners: &[DeclarationReference],
    parameters: &[ImplementationParameterId],
    passes: usize,
    shared: bool,
) -> Observation {
    source.reset(None);
    let mut used = 0;
    let mut analysis = analysis(source, &mut used);
    let mut scopes = Scopes::default();
    let start = Instant::now();
    for _ in 0..passes {
        for (rotation, &owner) in owners.iter().enumerate() {
            for (position, &parameter) in parameters.iter().enumerate().rev() {
                let ordinal = if shared {
                    scopes.ordinal(&mut analysis, owner, parameter).unwrap()
                } else {
                    // Literal predecessor operation: do not let the changed
                    // projection supply this comparison's answer or source reads.
                    let declaration = analysis.declaration(owner).unwrap();
                    analysis
                        .parameter_ordinal(&declaration.payload, parameter)
                        .unwrap()
                };
                let width = parameters.len();
                assert_eq!(ordinal, (position + width - rotation % width) % width);
                std::hint::black_box(ordinal);
            }
        }
    }
    let elapsed_ns = start.elapsed().as_nanos();
    if shared {
        assert_eq!(scopes.entries.len(), owners.len());
        assert_eq!(source.reads.get(), owners.len());
        assert_eq!(
            scopes.entries.values().map(Vec::len).sum::<usize>(),
            owners.len() * parameters.len()
        );
    } else {
        assert!(scopes.entries.is_empty());
        assert_eq!(source.reads.get(), owners.len() * parameters.len() * passes);
    }
    Observation {
        reads: source.reads.get(),
        work: *analysis.work,
        metadata: analysis.observation.metadata_bytes,
        elapsed_ns,
    }
}

#[test]
fn repeated_lexical_lookups_share_only_complete_ordered_id_projections() {
    for width in [1, 8, 64, 256] {
        let parameters = ids(width);
        // Equal declaration IDs in different packages, plus different lexical
        // owners in one package, deliberately have different authored ordinals.
        let owners = [owner(1, 1), owner(2, 1), owner(1, 2), owner(2, 2)];
        let mut source = Source::default();
        for (rotation, &owner) in owners.iter().enumerate() {
            let mut ordered = parameters.clone();
            ordered.rotate_left(rotation % width);
            source.add(owner, &ordered);
        }
        for passes in [1, 8] {
            // Alternate order; elapsed observations are not a speed acceptance
            // criterion and cannot establish a whole-program causal speedup.
            let (baseline, candidate) = if passes == 1 {
                let candidate = measure(&source, &owners, &parameters, passes, true);
                let baseline = measure(&source, &owners, &parameters, passes, false);
                (baseline, candidate)
            } else {
                let baseline = measure(&source, &owners, &parameters, passes, false);
                let candidate = measure(&source, &owners, &parameters, passes, true);
                (baseline, candidate)
            };
            eprintln!(
                "lexical-lookup width={width} owners={} passes={passes} baseline_reads={} candidate_reads={} baseline_work={} candidate_work={} baseline_metadata={} candidate_metadata={} baseline_ns={} candidate_ns={}",
                owners.len(),
                baseline.reads,
                candidate.reads,
                baseline.work,
                candidate.work,
                baseline.metadata,
                candidate.metadata,
                baseline.elapsed_ns,
                candidate.elapsed_ns
            );
        }
    }
}
