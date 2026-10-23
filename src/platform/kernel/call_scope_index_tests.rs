use super::*;

fn scope(ordinal: u64) -> ReadScopeReference {
    ReadScopeReference {
        binder: ReadScopeBinder::Declaration(reference(ordinal)),
        parameter: crate::platform::semantic_id::ReadScopeParameterId::migrate(SEED, ordinal),
    }
}

// Retain the previous two-search path as a bounded cost/behavior comparison, not
// as an oracle for lexical permission. Fixtures construct exact maps independently.
fn previous(
    analysis: &mut Analysis<'_, Read>,
    key: (usize, usize, Option<ExpressionId>),
    scopes: BTreeMap<ReadScopeReference, ReadScopeArgument>,
) -> Result<(), Diagnostic> {
    analysis.comparison_work(scopes.len() + 3, analysis.associated.call_scopes.len())?;
    if let Some(retained) = analysis.associated.call_scopes.get(&key) {
        return if retained == &scopes {
            Ok(())
        } else {
            Err(associated::invalid())
        };
    }
    analysis.reserve::<(
        (usize, usize, Option<ExpressionId>),
        BTreeMap<ReadScopeReference, ReadScopeArgument>,
    )>(1)?;
    analysis.associated.call_scopes.insert(key, scopes);
    Ok(())
}

#[test]
fn composed_call_scope_index_preserves_exact_inventory_with_one_descent() {
    let read = Read::default();
    let (mut work, mut old_work) = (0, 0);
    let mut current = root_query_analysis(&read, &mut work, 10_000_000);
    let mut old = root_query_analysis(&read, &mut old_work, 10_000_000);
    let mut expected = BTreeMap::new();
    let mut ordered = Vec::new();
    for index in 0..4096 {
        let expression = Some(ExpressionId::migrate(SEED, index as u64));
        let key = (index % 5, index % 13, expression);
        let scopes = if index % 4 == 0 {
            BTreeMap::from([(scope(1), ReadScopeArgument::Parameter(scope(2)))])
        } else {
            BTreeMap::new()
        };
        expected.insert(key, scopes.clone());
        ordered.push((key, scopes.clone()));
        previous(&mut old, key, scopes.clone()).unwrap();
        current
            .retain_call_scopes(key.0, key.1, key.2, scopes)
            .unwrap();
    }
    assert_eq!(current.associated.call_scopes, expected);
    assert_eq!(old.associated.call_scopes, expected);
    assert_eq!(
        current.observation.metadata_bytes,
        old.observation.metadata_bytes
    );
    let accepted_work = *current.work;
    assert_eq!(*old.work, 2 * accepted_work);
    assert!(
        accepted_work < 2_500_000,
        "bounded 4096-site regression fixture"
    );
    eprintln!(
        "call-scope index sites=4096 previous_work={} current_work={} metadata_bytes={}",
        old.work, current.work, current.observation.metadata_bytes
    );
    for (key, scopes) in &ordered {
        current
            .retain_call_scopes(key.0, key.1, key.2, scopes.clone())
            .unwrap();
    }
    assert_eq!(current.associated.call_scopes, expected);
    assert_eq!(
        current.observation.metadata_bytes,
        old.observation.metadata_bytes
    );
    let mut bounded_work = 0;
    let mut bounded = root_query_analysis(&read, &mut bounded_work, accepted_work);
    for (key, scopes) in &ordered {
        bounded
            .retain_call_scopes(key.0, key.1, key.2, scopes.clone())
            .unwrap();
    }
    assert_eq!(bounded.associated.call_scopes, expected);
    let mut refused_work = 0;
    let mut refused = root_query_analysis(&read, &mut refused_work, accepted_work);
    assert!(ordered.iter().any(|(key, scopes)| {
        previous(&mut refused, *key, scopes.clone())
            .is_err_and(|error| error.class == DiagnosticClass::Resource)
    }));
}

#[test]
fn composed_call_scope_index_refusals_do_not_expose_new_membership() {
    let read = Read::default();
    let mut work = 0;
    let mut analysis = root_query_analysis(&read, &mut work, 100_000);
    let key = (2, 3, Some(ExpressionId::migrate(SEED, 9)));
    analysis.maximum_metadata_bytes = 0;
    let failure = analysis
        .retain_call_scopes(key.0, key.1, key.2, BTreeMap::new())
        .unwrap_err();
    assert_eq!(failure.code, "kernel_callable_flow_storage");
    assert!(analysis.associated.call_scopes.is_empty());
    analysis.maximum_metadata_bytes = MAXIMUM_ANALYSIS_BYTES;
    analysis.maximum_work = Some(*analysis.work);
    let failure = analysis
        .retain_call_scopes(key.0, key.1, key.2, BTreeMap::new())
        .unwrap_err();
    assert_eq!(failure.code, "kernel_callable_flow_work");
    assert!(analysis.associated.call_scopes.is_empty());
    analysis.maximum_work = Some(100_000);
    read.cancel_after.set(Some(read.checkpoints.get() + 1));
    let failure = analysis
        .retain_call_scopes(key.0, key.1, key.2, BTreeMap::new())
        .unwrap_err();
    assert_eq!(failure.class, DiagnosticClass::Cancelled);
    assert!(analysis.associated.call_scopes.is_empty());
    read.cancel_after.set(None);
    analysis
        .retain_call_scopes(key.0, key.1, key.2, BTreeMap::new())
        .unwrap();
    let metadata = analysis.observation.metadata_bytes;
    analysis.maximum_metadata_bytes = metadata;
    analysis
        .retain_call_scopes(key.0, key.1, key.2, BTreeMap::new())
        .unwrap();
    let changed = BTreeMap::from([(scope(1), ReadScopeArgument::Parameter(scope(2)))]);
    let failure = analysis
        .retain_call_scopes(key.0, key.1, key.2, changed)
        .unwrap_err();
    assert_eq!(failure.class, DiagnosticClass::Semantic);
    assert_eq!(analysis.associated.call_scopes.len(), 1);
    assert!(analysis.associated.call_scopes[&key].is_empty());
    assert_eq!(analysis.observation.metadata_bytes, metadata);
}
