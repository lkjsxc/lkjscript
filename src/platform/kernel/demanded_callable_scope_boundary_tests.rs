use super::*;

fn sample() -> (Source, DeclarationReference, ImplementationParameterId) {
    let ids = ids(8);
    let owner = owner(1, 1);
    let mut source = Source::default();
    source.add(owner, &ids);
    (source, owner, ids[7])
}

fn cold_cost(
    source: &Source,
    owner: DeclarationReference,
    id: ImplementationParameterId,
) -> (usize, usize, usize) {
    source.reset(None);
    let mut work = 0;
    let mut analysis = analysis(source, &mut work);
    Scopes::default().ordinal(&mut analysis, owner, id).unwrap();
    (
        *analysis.work,
        analysis.observation.metadata_bytes,
        source.checkpoints.get(),
    )
}

#[test]
fn cold_scope_projection_keeps_exact_work_and_metadata_admission() {
    let (source, owner, id) = sample();
    let (work, bytes, _) = cold_cost(&source, owner, id);
    assert!(work > 0 && bytes > 0);
    for extra in [0, 1] {
        source.reset(None);
        let mut used = 0;
        let mut analysis = analysis(&source, &mut used);
        analysis.maximum_work = work - extra;
        let mut scopes = Scopes::default();
        let result = scopes.ordinal(&mut analysis, owner, id);
        if extra == 0 {
            assert_eq!(result.unwrap(), 7);
            assert_eq!(*analysis.work, work);
            assert_eq!(scopes.entries.len(), 1);
        } else {
            assert_eq!(result.unwrap_err().class, DiagnosticClass::Resource);
            assert!(scopes.entries.is_empty());
        }
    }
    for extra in [0, 1] {
        source.reset(None);
        let mut used = 0;
        let mut analysis = analysis(&source, &mut used);
        analysis.observation.metadata_bytes = MAXIMUM_ANALYSIS_BYTES - bytes + extra;
        let mut scopes = Scopes::default();
        let result = scopes.ordinal(&mut analysis, owner, id);
        if extra == 0 {
            assert_eq!(result.unwrap(), 7);
            assert_eq!(analysis.observation.metadata_bytes, MAXIMUM_ANALYSIS_BYTES);
        } else {
            assert_eq!(result.unwrap_err().class, DiagnosticClass::Resource);
            assert!(scopes.entries.is_empty());
        }
    }
    source.reset(None);
    let mut used = 0;
    let mut analysis = analysis(&source, &mut used);
    assert_eq!(
        Scopes::default().ordinal(&mut analysis, owner, id).unwrap(),
        7
    );
}

#[test]
fn every_cold_projection_checkpoint_refuses_without_a_partial_entry() {
    let (source, owner, id) = sample();
    let (_, _, checkpoints) = cold_cost(&source, owner, id);
    assert!(checkpoints > 3);
    for cutoff in 1..=checkpoints {
        source.reset(Some(cutoff));
        let mut used = 0;
        let mut analysis = analysis(&source, &mut used);
        let mut scopes = Scopes::default();
        let error = scopes.ordinal(&mut analysis, owner, id).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Cancelled);
        assert_eq!(error.code, "scope_projection_cancelled");
        assert!(scopes.entries.is_empty(), "cutoff={cutoff}");
        source.reset(None);
        assert_eq!(
            Scopes::default().ordinal(&mut analysis, owner, id).unwrap(),
            7
        );
    }
}

#[test]
fn warm_scope_hits_keep_cancellation_work_and_the_original_projection() {
    let (source, owner, id) = sample();
    let mut used = 0;
    let mut analysis = analysis(&source, &mut used);
    let mut scopes = Scopes::default();
    scopes.ordinal(&mut analysis, owner, id).unwrap();
    let bytes = analysis.observation.metadata_bytes;
    source.reset(None);
    let before = *analysis.work;
    assert_eq!(scopes.ordinal(&mut analysis, owner, id).unwrap(), 7);
    let hit_work = *analysis.work - before;
    let checkpoints = source.checkpoints.get();
    assert_eq!(source.reads.get(), 0);
    for cutoff in 1..=checkpoints {
        source.reset(Some(cutoff));
        let error = scopes.ordinal(&mut analysis, owner, id).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Cancelled);
        assert_eq!(error.code, "scope_projection_cancelled");
        assert_eq!(scopes.entries.len(), 1);
        source.reset(None);
        assert_eq!(scopes.ordinal(&mut analysis, owner, id).unwrap(), 7);
        assert_eq!(source.reads.get(), 0);
        assert_eq!(analysis.observation.metadata_bytes, bytes);
    }
    source.reset(None);
    analysis.maximum_work = *analysis.work + hit_work - 1;
    let error = scopes.ordinal(&mut analysis, owner, id).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(scopes.entries.len(), 1);
    analysis.maximum_work = 10_000_000;
    assert_eq!(scopes.ordinal(&mut analysis, owner, id).unwrap(), 7);
    assert_eq!(source.reads.get(), 0);
    assert_eq!(analysis.observation.metadata_bytes, bytes);
}
