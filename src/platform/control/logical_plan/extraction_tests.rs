//! Authored extraction through the review-bound logical-plan pipeline.

use super::*;
use crate::platform::builtin_standard::BuiltinStandard;
use crate::platform::control::{NormalizedChangeRequest, decode_compact_change};
use crate::platform::kernel::{
    DeclarationPayload, ExpressionOperation, RequirementOperand, RequirementReference,
};
use crate::platform::project_creation::{ProjectTemplate, create_project};
use crate::platform::publication::GraphRepository;
use std::io::Cursor;

fn prepare_extraction(
    repository: &GraphRepository,
    request: &NormalizedChangeRequest,
) -> PreparedAuthoredPublication {
    let mut prepared = repository
        .prepare_authored_change(&request.semantic, request.options.clone())
        .unwrap();
    let extraction = prepared.logical_plan.extraction.as_mut().unwrap();
    // Match the public CLI boundary's binding from the pinned accepted definition.
    extraction.base_definition = Some(
        crate::platform::cli::function_definition_digest_for_extraction(
            &repository.view_current().unwrap(),
            extraction.function,
        )
        .unwrap(),
    );
    prepared
}

fn round_trip(
    request: &NormalizedChangeRequest,
    prepared: &PreparedAuthoredPublication,
) -> (LogicalPlanEncoding, Vec<CompactRecord>) {
    let plan = LogicalChangePlan::new(request.request_commitment, prepared).unwrap();
    let mut bytes = Vec::new();
    let encoding = encode_logical_change_plan(&plan, |part| {
        bytes.extend_from_slice(part);
        Ok(())
    })
    .unwrap();
    let decoded = decode_logical_change_plan(Cursor::new(&bytes)).unwrap();
    assert_eq!(decoded.token, encoding.token.to_string());
    assert_eq!(
        decoded.request_commitment,
        request.request_commitment.to_string(),
    );
    assert_eq!(decoded.bytes, encoding.bytes);
    assert_eq!(decoded.records, encoding.records);
    assert_eq!(decoded.counts.extractions, 1);
    assert_eq!(
        decoded.counts.extraction_requirements,
        extraction_requirement_count(prepared.logical_plan.extraction.as_ref().unwrap()) as u64,
    );
    (
        encoding,
        parse_records("extraction logical plan", &bytes).unwrap(),
    )
}

#[test]
fn authored_extraction_plan_round_trips_task_kind_rows_and_exact_moved_identities() {
    let standard = BuiltinStandard::load().unwrap();
    for (selected_source, result_type, kind, uses_clock, moved_count) in [
        ("(call child (i64 17))", "I64", "task", false, 2),
        (
            "(parallel (call child (i64 17)) (call child (i64 -29)))",
            "(record (left I64) (right I64))",
            "task",
            false,
            5,
        ),
        (
            "(invoke (function-value child) (i64 17))",
            "I64",
            "task",
            false,
            3,
        ),
        ("(i64 17)", "I64", "pure", false, 1),
        ("(call read-clock)", "I64", "task", true, 1),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let project = temporary.path().join("project");
        create_project(&project, "logical-extraction", ProjectTemplate::Minimal).unwrap();
        let repository = GraphRepository::open(&project).unwrap();
        repository
            .stage_package_transport(standard.package_transport, standard.transport_bytes())
            .unwrap();
        let caller_effect = if uses_clock || selected_source.starts_with("(parallel") {
            "(task (requirement authority::clock) (requirement authority::unused))"
        } else if kind == "pure" {
            "pure"
        } else {
            "(task)"
        };
        let literal = format!(
            r#"request base={}
add.dependency package={} semantic-revision={} package-revision={}
declarations.begin
(units (use std builtin) (module create extraction
  (function create available (visibility private) (returns I64)
    (effect pure) (body (i64 0)))
  (component create authority (visibility public)
    (requirement create clock (interface std::WallClock)
      (operations std::WallClock::utc-milliseconds) (limits (maximum_calls 1 calls)))
    (requirement create unused (interface std::WallClock)
      (operations std::WallClock::utc-milliseconds) (limits (maximum_calls 1 calls)))
    (port create ready (type (function () I64)) (function available)))
  (function create read-clock (visibility public) (returns I64)
    (effect (task (requirement authority::clock)))
    (body (capability-call authority::clock std::WallClock::utc-milliseconds)))
  (function create child (visibility public) (parameter create n (type I64))
    (returns I64) (effect (task)) (body (local n)))
  (function create parent (visibility public) (returns {result_type})
    (effect {caller_effect})
    (body (sequence (unit) {selected_source})))))
declarations.end
"#,
            repository.view_current().unwrap().revision(),
            standard.package,
            standard.semantic_revision,
            standard.package_revision,
        );
        if selected_source.starts_with("(parallel") {
            let invalid_literal = literal.replace("(call child (i64 17))", "(call read-clock)");
            let invalid = decode_compact_change(
                "effectful-parallel-extraction-fixture",
                invalid_literal.as_bytes(),
            )
            .unwrap();
            let base = repository.view_current().unwrap().revision();
            let errors = repository
                .prepare_authored_change(&invalid.semantic, invalid.options)
                .unwrap_err();
            assert!(
                errors
                    .iter()
                    .any(|error| error.code == "kernel_parallel_call")
            );
            assert_eq!(repository.view_current().unwrap().revision(), base);
        }
        let fixture =
            decode_compact_change("logical-extraction-fixture", literal.as_bytes()).unwrap();
        let fixture = repository
            .prepare_authored_change(&fixture.semantic, fixture.options)
            .unwrap();
        repository.publish(&fixture.publication).unwrap();
        let before = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let (function, selected) = before
            .owners
            .iter()
            .find_map(|(key, owner)| {
                let (OwnerKey::Declaration(function), OwnerRecord::Declaration(d)) = (key, owner)
                else {
                    return None;
                };
                if d.name.as_str() != "parent" {
                    return None;
                }
                let DeclarationPayload::Function(f) = &d.payload else {
                    return None;
                };
                let OwnerRecord::Expression(e) = &before.owners[&OwnerKey::Expression(f.body)]
                else {
                    return None;
                };
                let ExpressionOperation::Sequence { items } = &e.operation else {
                    return None;
                };
                Some((*function, items[1]))
            })
            .unwrap();
        // Inventory the accepted subtree before extraction. These fixtures have only expressions,
        // no captures; their independently specified node counts guard the expected closure.
        let mut moved = BTreeSet::new();
        let mut pending = vec![selected];
        while let Some(id) = pending.pop() {
            assert!(moved.insert(OwnerKey::Expression(id)));
            let OwnerRecord::Expression(e) = &before.owners[&OwnerKey::Expression(id)] else {
                panic!("accepted expression")
            };
            pending.extend(e.children().into_iter().map(|child| child.expression));
        }
        assert_eq!(moved.len(), moved_count);
        let requirements = before
            .owners
            .iter()
            .filter_map(|(key, owner)| {
                let (OwnerKey::Requirement(id), OwnerRecord::Requirement(record)) = (key, owner)
                else {
                    return None;
                };
                (uses_clock && record.name.as_str() == "clock").then_some(
                    RequirementOperand::Concrete(RequirementReference {
                        package: before.root.package_id,
                        requirement: *id,
                    }),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(requirements.len(), usize::from(uses_clock));
        let literal = format!(
            "request base={}\nextract.function as=$helper function={function} expression={selected} name=extracted\n",
            repository.view_current().unwrap().revision(),
        );
        let request = decode_compact_change("logical-extraction", literal.as_bytes()).unwrap();
        let prepared = prepare_extraction(&repository, &request);
        let extraction = prepared.logical_plan.extraction.as_ref().unwrap();
        let expected_effect = if kind == "pure" {
            FunctionEffect::Pure
        } else {
            FunctionEffect::Task {
                effect_parameters: Vec::new(),
                requirements: requirements.clone(),
            }
        };
        assert_eq!(extraction.effect, expected_effect, "{selected_source}");
        assert!(extraction.captures.is_empty());
        let mut moved = moved.into_iter().collect::<Vec<_>>();
        moved.sort_unstable_by_key(|owner| EncodedOwnerKey::new(*owner));
        assert_eq!(extraction.moved_owners, moved);
        assert_eq!(extraction.preserved_owners, moved);
        let (encoding, records) = round_trip(&request, &prepared);
        let header = records
            .iter()
            .find(|r| r.operation == "logical-plan.extraction")
            .unwrap();
        assert_eq!(field(header, 8), kind);
        assert_eq!(field(header, 12), requirements.len().to_string());
        let encoded_moved = records
            .iter()
            .filter(|r| r.operation == "logical-plan.extraction-owner" && field(r, 0) == "moved")
            .map(|r| field(r, 2).parse::<OwnerKey>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(encoded_moved, moved);
        if uses_clock {
            let mut invalid = prepared.clone();
            let FunctionEffect::Task { requirements, .. } =
                &mut invalid.logical_plan.extraction.as_mut().unwrap().effect
            else {
                panic!("effectful task")
            };
            requirements.push(requirements[0]);
            let error = LogicalChangePlan::new(request.request_commitment, &invalid)
                .err()
                .unwrap();
            assert_eq!(error.code, "change_logical_plan_extraction_requirements");

            let mut invalid = records.clone();
            let header = invalid
                .iter_mut()
                .find(|r| r.operation == "logical-plan.extraction")
                .unwrap();
            header.fields[8].value = "pure".to_owned();
            assert_eq!(
                decode_records(&invalid).unwrap_err().code,
                "change_plan_file_extraction_effect",
            );
            let mut missing = records.clone();
            missing.retain(|r| r.operation != "logical-plan.extraction-requirement");
            let counts = missing
                .iter_mut()
                .find(|r| r.operation == "logical-plan.counts")
                .unwrap();
            counts.fields[14].value = "0".to_owned();
            assert_eq!(
                decode_records(&missing).unwrap_err().code,
                "change_plan_file_extraction_counts",
            );
        }
        // Reprepare the same literal request before applying, as reviewed public apply does.
        let reprepared = prepare_extraction(&repository, &request);
        assert_eq!(round_trip(&request, &reprepared).0.token, encoding.token);
        repository.publish(&reprepared.publication).unwrap();
        let after = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let OwnerRecord::Declaration(helper) =
            &after.owners[&OwnerKey::Declaration(extraction.helper)]
        else {
            panic!("extracted helper")
        };
        let DeclarationPayload::Function(helper) = &helper.payload else {
            panic!("extracted helper function")
        };
        assert_eq!(helper.body, selected);
        assert_eq!(helper.effect, expected_effect);
        for owner in &moved {
            assert_eq!(
                after.owners[owner], before.owners[owner],
                "moved owner {owner}",
            );
        }
    }
}

fn decode_records(records: &[CompactRecord]) -> Result<DecodedLogicalPlan, Diagnostic> {
    let mut bytes = Vec::new();
    for record in records {
        let fields = record
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.value.as_str()))
            .collect::<Vec<_>>();
        bytes.extend_from_slice(
            render_record(&record.operation, &fields)
                .unwrap()
                .as_bytes(),
        );
    }
    decode_logical_change_plan(Cursor::new(bytes))
}
