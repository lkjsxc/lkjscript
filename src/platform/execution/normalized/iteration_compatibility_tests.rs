//! Exact Graph 14 predecessor admission and owner-local iteration cutover.
use super::{NormalizedReferenceInterpreter, NormalizedVm};
use crate::platform::kernel::*;
use crate::platform::package_transport::source::PackageContainer;
use crate::platform::publication::GraphRepository;
use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

#[test]
#[ignore = "explicit authenticated predecessor inventory acquisition"]
fn retain_task_iteration_predecessor_inventory() {
    let source = std::env::var_os("LKJSCRIPT_ITERATION_PREDECESSOR_SOURCE").unwrap();
    let application = GraphRepository::open(&Path::new(&source).join("applications/lkjournal"))
        .unwrap()
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let proof = serde_json::json!({"source_commit":"67baaf0b081842e0e2e3745e8d5503e22cc791e4","standard_transport_sha256":"83c053e6575b29b1fd759d225fe967a7e4d9e8c79dbf440cd515efabe93efa3a","standard_artifact_sha256":"abdade8cbf2e075e6bff02895bd2babde63f41fca4dbc93db20cd0a1d3565da9","application_package":application.root.package_id,"application_owners":application.root.owners});
    std::fs::write(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/task-iteration-predecessor.json"),
        serde_json::to_vec_pretty(&proof).unwrap(),
    )
    .unwrap();
}

fn function(snapshot: &KernelSnapshot, name: &str) -> (OwnerKey, FunctionDeclaration) {
    snapshot
        .owners
        .iter()
        .find_map(|(id, r)| {
            if let OwnerRecord::Declaration(d) = r {
                if d.name.as_str() == name {
                    if let DeclarationPayload::Function(f) = &d.payload {
                        Some((*id, f.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap()
}

#[test]
fn task_iteration_cutover_preserves_unrelated_owners_public_contracts_and_old_supported_artifacts()
{
    let bytes = include_bytes!("../../../../tests/fixtures/graph14-before-task-iteration.lkjp");
    let transport = PackageTransportDigest::from_str(
        "package_transport_76acdf9341178a1d49125e3c067fed5633113d8e30ddec34339b319365a4dfcb",
    )
    .unwrap();
    let closure = PackageContainer::decode(bytes, transport)
        .unwrap()
        .admit()
        .unwrap();
    let old = &closure.packages[&closure.container.root.package_revision].snapshot;
    let current =
        GraphRepository::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard"))
            .unwrap()
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
    assert_eq!(old.root.package_id, current.root.package_id);
    let (fold, old_fold) = function(old, "task-fold-left");
    let (range, old_range) = function(old, "task-fold-left-range");
    let (_, mut new_fold) = function(&current, "task-fold-left");
    new_fold.body = old_fold.body;
    assert_eq!(
        old_fold, new_fold,
        "fold signature or parameter identities changed"
    );
    assert_eq!(function(old, "task-map"), function(&current, "task-map"));
    assert!(!current.owners.contains_key(&range));
    let mut obsolete = BTreeSet::from([range]);
    obsolete.extend(
        old_range
            .type_parameters
            .iter()
            .map(|p| OwnerKey::TypeParameter(*p)),
    );
    obsolete.extend(
        old_range
            .effect_parameters
            .iter()
            .map(|p| OwnerKey::EffectParameter(*p)),
    );
    obsolete.extend(old_range.parameters.iter().map(|p| OwnerKey::Parameter(*p)));
    let mut pending = vec![old_fold.body, old_range.body];
    while let Some(id) = pending.pop() {
        if obsolete.insert(OwnerKey::Expression(id)) {
            let OwnerRecord::Expression(e) = &old.owners[&OwnerKey::Expression(id)] else {
                panic!("expression")
            };
            pending.extend(e.children().into_iter().map(|c| c.expression));
            if let ExpressionOperation::Let { bindings, .. } = &e.operation {
                for binding in bindings {
                    obsolete.insert(OwnerKey::Binding(*binding));
                    pending.extend(old.owners[&OwnerKey::Binding(*binding)].expression_roots());
                }
            }
        }
    }
    let mut preserved = 0;
    for (id, owner) in &old.owners {
        if *id != fold && !obsolete.contains(id) {
            assert_eq!(
                current.owners.get(id),
                Some(owner),
                "unrelated owner {id} changed"
            );
            assert_eq!(
                encode_owner(owner).unwrap(),
                encode_owner(&current.owners[id]).unwrap()
            );
            preserved += 1;
        }
    }
    for (id, ty) in &old.types {
        if let Some(after) = current.types.get(id) {
            assert_eq!(
                encode_type_object(ty).unwrap(),
                encode_type_object(after).unwrap()
            );
        }
    }
    let loaded = crate::platform::compiler::load_artifact(include_bytes!(
        "../../../../tests/fixtures/graph14-before-task-iteration.lkja"
    ))
    .unwrap();
    let program =
        crate::platform::execution::normalized::NormalizedProgram::prepare(loaded).unwrap();
    let control = crate::platform::execution::ExecutionControl::uncancelled();
    let vm = NormalizedVm::new(&program, Default::default());
    let reference = NormalizedReferenceInterpreter::new(old, &program, Default::default());
    let mut tests = 0;
    for target in program.tests.values() {
        let (actual, expected) = vm.invoke_test(target.declaration, None, &control).unwrap();
        assert_eq!(actual.0, expected.0);
        let (actual_ref, expected_ref) = reference
            .invoke_test(target.declaration, None, &control)
            .unwrap();
        assert_eq!(actual_ref.0, expected_ref.0);
        assert_eq!(actual.0, actual_ref.0);
        tests += 1;
    }
    assert_eq!(tests, 33);
    let proof: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/task-iteration-predecessor.json"
    ))
    .unwrap();
    let application = GraphRepository::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal"),
    )
    .unwrap()
    .view_current()
    .unwrap()
    .reconstruct_full_oracle()
    .unwrap()
    .value;
    assert_eq!(
        serde_json::to_value(application.root.owners).unwrap(),
        proof["application_owners"]
    );
    assert_eq!(
        serde_json::to_value(application.root.package_id).unwrap(),
        proof["application_package"]
    );
    println!(
        "iteration compatibility: {preserved} unrelated standard owner records byte-identical; {tests} predecessor tests equal in both evaluators; all application owner-map bytes retained"
    );
}
