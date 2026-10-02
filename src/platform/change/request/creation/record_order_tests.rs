//! Literal public authoring fixtures prove initializer sequencing independently of layout.
//! Runtime adapters are private to their owner, so this fixture inspects strictly admitted
//! compiled effect sites rather than substituting a hand-written expression evaluator.

use super::*;
use crate::platform::compiler::{
    CompilationPayload, CompilationUnit, CompiledInstruction, OptimizationPolicy, build_clean,
    link_artifact, load_artifact,
};
use crate::platform::control::{NormalizedChangeRequest, decode_compact_change};
use crate::platform::kernel::{KernelSnapshot, OperationReference};
use crate::platform::publication::{GraphRepository, PreparedAuthoredPublication};
use crate::platform::storage::object::ObjectDomain;
use std::collections::BTreeMap;

const DECLARATIONS: &str = r#"
create.module as=$module name=record-order
create.interface as=$Transcript module=$module name=Transcript visibility=public
add.operation as=$emitZ interface=$Transcript name=emitZ result=i64 idempotency=non-idempotent external-visibility=none
add.operation as=$emitA interface=$Transcript name=emitA result=i64 idempotency=non-idempotent external-visibility=none
create.component as=$component module=$module name=TranscriptHost visibility=public
add.requirement as=$trace component=$component name=trace interface=$Transcript
requirement.operation parent=$trace index=0 operation=$emitZ
requirement.operation parent=$trace index=1 operation=$emitA

expression.block as=$mark-z-body
(capability-call $trace $emitZ)
expression.end
create.function as=$markZ module=$module name=markZ visibility=private result=i64 effect=task body=$mark-z-body
effect.requirement parent=$markZ index=0 requirement=$trace
expression.block as=$mark-a-body
(capability-call $trace $emitA)
expression.end
create.function as=$markA module=$module name=markA visibility=private result=i64 effect=task body=$mark-a-body
effect.requirement parent=$markA index=0 requirement=$trace

type.structural-record as=@Pair
type.field parent=@Pair index=0 name=a type=i64
type.field parent=@Pair index=1 name=z type=i64
create.record as=$NominalPair module=$module name=NominalPair visibility=private
add.type-parameter as=$Item declaration=$NominalPair name=Item
type.parameter as=@Item parameter=$Item
add.field as=$nominalZ record=$NominalPair name=z type=@Item
add.field as=$nominalA record=$NominalPair name=a type=@Item
type.application as=@Nominal declaration=$NominalPair
type.argument parent=@Nominal index=0 type=i64

create.function as=$structuralZA module=$module name=structuralZA visibility=private result=@Pair effect=task body=$structural-za
effect.requirement parent=$structuralZA index=0 requirement=$trace
expression.block as=$structural-az
(record structural (field a (call $markA)) (field z (call $markZ)))
expression.end
create.function as=$structuralAZ module=$module name=structuralAZ visibility=private result=@Pair effect=task body=$structural-az
effect.requirement parent=$structuralAZ index=0 requirement=$trace
expression.block as=$nominal-za
(record $NominalPair (types i64) (field $nominalZ (call $markZ)) (field $nominalA (call $markA)))
expression.end
create.function as=$nominalZA module=$module name=nominalZA visibility=private result=@Nominal effect=task body=$nominal-za
effect.requirement parent=$nominalZA index=0 requirement=$trace
expression.block as=$nominal-az
(record $NominalPair (types i64) (field $nominalA (call $markA)) (field $nominalZ (call $markZ)))
expression.end
create.function as=$nominalAZ module=$module name=nominalAZ visibility=private result=@Nominal effect=task body=$nominal-az
effect.requirement parent=$nominalAZ index=0 requirement=$trace

effect.row as=@Trace
effect.requirement parent=@Trace index=0 requirement=$trace
type.task-function as=@Entry result=@Pair effect=@Trace
add.port as=$entry component=$component name=entry type=@Entry function=$structuralZA
"#;

const STRUCTURAL_ZA: &str = r#"
expression.block as=$structural-za
(record structural (field z (call $markZ)) (field a (call $markA)))
expression.end
"#;

// Independently authored indexed form; it must preserve the same initializer order.
const FLAT_ZA: &str = r#"
expression.record as=$structural-za
expression.record-field parent=$structural-za index=0 name=z value=$z-call
expression.record-field parent=$structural-za index=1 name=a value=$a-call
expression.call as=$z-call function=$markZ
expression.call as=$a-call function=$markA
"#;

fn request(repository: &GraphRepository, root: &str) -> NormalizedChangeRequest {
    let source = format!(
        "request base={}\n{root}{DECLARATIONS}",
        repository.view_current().unwrap().revision(),
    );
    decode_compact_change("record-order.lkjc", source.as_bytes()).unwrap()
}

fn prepare(
    repository: &GraphRepository,
    request: &NormalizedChangeRequest,
) -> PreparedAuthoredPublication {
    repository
        .prepare_authored_change(&request.semantic, request.options.clone())
        .unwrap_or_else(|errors| panic!("record-order candidate: {errors:#?}"))
}

fn declaration(prepared: &PreparedAuthoredPublication, symbol: &str) -> DeclarationId {
    let OwnerKey::Declaration(id) = prepared.allocated[symbol] else {
        panic!("declaration {symbol}")
    };
    id
}

fn expression(
    snapshot: &KernelSnapshot,
    id: crate::platform::semantic_id::ExpressionId,
) -> &ExpressionOperation {
    let OwnerRecord::Expression(record) = &snapshot.owners[&OwnerKey::Expression(id)] else {
        panic!("expression")
    };
    &record.operation
}

fn record_body<'a>(
    snapshot: &'a KernelSnapshot,
    function: &FunctionDeclaration,
) -> (Vec<BindingId>, &'a ExpressionOperation) {
    match expression(snapshot, function.body) {
        ExpressionOperation::Let { bindings, body } => {
            (bindings.clone(), expression(snapshot, *body))
        }
        record => (Vec::new(), record),
    }
}

// This is a compiler fixture proof, not an execution or a simulation. These literal functions
// contain no branches, loops, indirect calls or arguments, so their effect-site order is exact.
fn compiled_effect_sites(
    units: &BTreeMap<DeclarationId, CompilationUnit>,
    declaration: DeclarationId,
) -> Vec<OperationReference> {
    let unit = &units[&declaration];
    let CompilationPayload::Function { code, .. } = &unit.payload else {
        panic!("compiled function")
    };
    let mut sites = Vec::new();
    for instruction in &code.instructions {
        match instruction {
            CompiledInstruction::Call {
                function,
                arguments,
                ..
            } => {
                assert_eq!(*arguments, 0);
                sites.extend(compiled_effect_sites(
                    units,
                    unit.tables.declarations[*function as usize].declaration,
                ));
            }
            CompiledInstruction::Perform {
                operation,
                arguments,
                ..
            } => {
                assert_eq!(*arguments, 0);
                sites.push(unit.tables.operations[*operation as usize]);
            }
            CompiledInstruction::StoreLocal(_)
            | CompiledInstruction::LoadLocal { .. }
            | CompiledInstruction::Record { .. }
            | CompiledInstruction::I64(_)
            | CompiledInstruction::Drop
            | CompiledInstruction::Return => {}
            unexpected => panic!("unexpected instruction in straight-line fixture: {unexpected:?}"),
        }
    }
    sites
}

#[test]
fn public_record_initializers_preserve_effect_sites_and_canonical_types() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let block = request(&created.repository, STRUCTURAL_ZA);
    let flat = request(&created.repository, FLAT_ZA);
    assert_eq!(block.request_commitment, flat.request_commitment);
    let prepared = prepare(&created.repository, &block);
    let flat_prepared = prepare(&created.repository, &flat);
    assert_eq!(
        prepared.publication.objects,
        flat_prepared.publication.objects
    );
    assert_eq!(
        prepared.publication.head_bytes,
        flat_prepared.publication.head_bytes
    );
    created.repository.publish(&prepared.publication).unwrap();
    let view = created.repository.view_current().unwrap();
    let snapshot = view.reconstruct_full_oracle().unwrap().value;
    let mut inferred = BTreeMap::new();
    let mut nominal_wrappers = 0;
    for (symbol, expected_names) in [
        ("$structuralZA", ["markZ", "markA"]),
        ("$structuralAZ", ["markA", "markZ"]),
        ("$nominalZA", ["markZ", "markA"]),
        ("$nominalAZ", ["markA", "markZ"]),
    ] {
        let id = declaration(&prepared, symbol);
        let OwnerRecord::Declaration(owner) = &snapshot.owners[&OwnerKey::Declaration(id)] else {
            panic!("function declaration")
        };
        let DeclarationPayload::Function(function) = &owner.payload else {
            panic!("function payload")
        };
        let ty = crate::platform::kernel::infer_function_expression_type(
            &snapshot,
            id,
            function.body,
            &function.effect,
            &mut 0,
            10_000,
        )
        .unwrap();
        assert_eq!(ty, function.result);
        inferred.insert(symbol, ty);
        let (bindings, body) = record_body(&snapshot, function);
        let ExpressionOperation::Record {
            fields,
            nominal_type,
            type_arguments,
        } = body
        else {
            panic!("canonical record body")
        };
        assert!(
            fields
                .windows(2)
                .all(|pair| pair[0].selector < pair[1].selector)
        );
        if nominal_type.is_some() {
            assert_eq!(type_arguments.len(), 1);
            nominal_wrappers += usize::from(!bindings.is_empty());
        } else {
            assert!(type_arguments.is_empty());
        }
        let initializers = if bindings.is_empty() {
            fields.iter().map(|field| field.value).collect::<Vec<_>>()
        } else {
            assert_eq!(bindings.len(), 2);
            for field in fields {
                let ExpressionOperation::Local {
                    value: LocalValueReference::LexicalBinding(binding),
                } = expression(&snapshot, field.value)
                else {
                    panic!("record fields must only read retained initializers")
                };
                assert!(bindings.contains(binding));
            }
            bindings
                .iter()
                .map(|binding| {
                    let OwnerRecord::Binding(record) =
                        &snapshot.owners[&OwnerKey::Binding(*binding)]
                    else {
                        panic!("initializer binding")
                    };
                    assert_eq!(record.kind, crate::platform::kernel::BindingKind::Let);
                    record.value.unwrap()
                })
                .collect()
        };
        let marker_name = |value| {
            let ExpressionOperation::Call { function, .. } = expression(&snapshot, value) else {
                panic!("marker call")
            };
            snapshot.owners[&OwnerKey::Declaration(function.declaration)]
                .name()
                .unwrap()
                .as_str()
        };
        let names = initializers
            .iter()
            .map(|value| marker_name(*value))
            .collect::<Vec<_>>();
        assert_eq!(names, expected_names);
        for field in fields {
            let name = match &field.selector {
                FieldSelector::Structural(name) => name,
                FieldSelector::Nominal(reference) => snapshot.owners
                    [&OwnerKey::Field(reference.field)]
                    .name()
                    .unwrap(),
            };
            let initializer = match expression(&snapshot, field.value) {
                ExpressionOperation::Local {
                    value: LocalValueReference::LexicalBinding(binding),
                } => {
                    let OwnerRecord::Binding(record) =
                        &snapshot.owners[&OwnerKey::Binding(*binding)]
                    else {
                        panic!("field binding")
                    };
                    record.value.unwrap()
                }
                _ => field.value,
            };
            assert_eq!(
                marker_name(initializer),
                if name.as_str() == "z" {
                    "markZ"
                } else {
                    "markA"
                }
            );
        }
        if symbol == "$structuralZA" {
            assert_eq!(bindings.len(), 2);
        }
        if symbol == "$structuralAZ" {
            assert!(bindings.is_empty());
        }
    }
    assert_eq!(inferred["$structuralZA"], inferred["$structuralAZ"]);
    assert_eq!(inferred["$nominalZA"], inferred["$nominalAZ"]);
    assert_eq!(
        nominal_wrappers, 1,
        "exactly one reverse nominal order needs temporaries"
    );

    let compiled = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    let linked = link_artifact(&created.repository, compiled.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&linked.artifact.bytes).unwrap();
    let units = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .filter_map(|(key, bytes)| {
            let unit = CompilationUnit::decode(bytes, *key).unwrap();
            let OwnerKey::Declaration(id) = unit.source.owner else {
                return None;
            };
            Some((id, unit))
        })
        .collect::<BTreeMap<_, _>>();
    let operation = |symbol: &str| {
        let OwnerKey::Operation(operation) = prepared.allocated[symbol] else {
            panic!("operation")
        };
        OperationReference {
            package: snapshot.root.package_id,
            operation,
        }
    };
    // Independent expected effect transcript: selectors/layout must never determine this order.
    for (symbol, expected) in [
        ("$structuralZA", [operation("$emitZ"), operation("$emitA")]),
        ("$structuralAZ", [operation("$emitA"), operation("$emitZ")]),
        ("$nominalZA", [operation("$emitZ"), operation("$emitA")]),
        ("$nominalAZ", [operation("$emitA"), operation("$emitZ")]),
    ] {
        assert_eq!(
            compiled_effect_sites(&units, declaration(&prepared, symbol)),
            expected
        );
    }
}

#[test]
fn record_temporaries_obey_identity_budget_without_publication() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let mut decoded = request(&created.repository, STRUCTURAL_ZA);
    let prepared = prepare(&created.repository, &decoded);
    decoded
        .semantic
        .budget
        .authored
        .maximum_allocated_identities = prepared.lowering_work.allocated_identities - 1;
    let before = created.repository.view_current().unwrap().revision();
    let errors = created
        .repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "change_budget_allocated_identities"),
        "{errors:#?}"
    );
    assert_eq!(
        created.repository.view_current().unwrap().revision(),
        before
    );
}

#[test]
fn nested_record_temporaries_share_the_authored_binding_domain() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let nested = request(
        &created.repository,
        r#"
expression.block as=$structural-za
(let (binding record-field (i64 7))
  (in (sequence
    (record structural (field z (call $markZ)) (field a (call $markA)))
    (record structural (field z (call $markZ)) (field a (call $markA))))))
expression.end
"#,
    );
    let prepared = prepare(&created.repository, &nested);
    let bindings = prepared
        .logical_plan
        .allocations
        .iter()
        .filter(|allocation| allocation.domain == crate::platform::kernel::IdentityKind::Binding)
        .collect::<Vec<_>>();
    // One authored binder, two reordered structural records, and one reordered nominal record.
    assert_eq!(bindings.len(), 7);
    assert_eq!(bindings[0].ordinal, 1);
    assert!(
        bindings
            .windows(2)
            .all(|pair| pair[1].ordinal == pair[0].ordinal + 1)
    );
    assert_eq!(
        bindings
            .iter()
            .map(|allocation| allocation.owner)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        bindings.len(),
    );

    let duplicate = request(
        &created.repository,
        r#"
expression.block as=$structural-za
(record structural (field z (call $markZ)) (field z (call $markA)))
expression.end
"#,
    );
    let errors = created
        .repository
        .prepare_authored_change(&duplicate.semantic, duplicate.options)
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "kernel_expression_record_duplicate"),
        "{errors:#?}",
    );
}
