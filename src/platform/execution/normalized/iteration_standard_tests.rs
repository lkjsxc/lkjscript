//! Current maintained standard behavior through both independent evaluators.
use super::{NormalizedReferenceInterpreter, NormalizedVm};
use crate::platform::builtin_standard::BuiltinStandard;
use crate::platform::compiler::load_artifact;
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{DeclarationPayload, DeclarationReference, OwnerKey, OwnerRecord};
use crate::platform::publication::GraphRepository;
use std::collections::BTreeSet;
use std::path::Path;

#[test]
fn current_standard_executes_every_authored_expected_test_in_both_evaluators() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard");
    let source = GraphRepository::open(&root)
        .unwrap()
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let artifact = std::fs::read(root.join("generated/standard.lkja")).unwrap();
    let loaded = load_artifact(&artifact).unwrap();
    let standard = BuiltinStandard::load().unwrap();
    assert_eq!(artifact.as_slice(), standard.artifact_bytes());
    assert_eq!(loaded.bundle_digest, standard.artifact.bundle_digest);
    let program = standard.prepared_program().unwrap();
    let authored = source
        .owners
        .iter()
        .filter_map(|(key, owner)| {
            let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) =
                (key, owner)
            else {
                return None;
            };
            matches!(record.payload, DeclarationPayload::Test { .. }).then_some(
                DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                },
            )
        })
        .collect::<BTreeSet<_>>();
    assert!(!authored.is_empty());
    let compiled = program
        .tests
        .values()
        .map(|test| test.declaration)
        .collect::<BTreeSet<_>>();
    assert_eq!(compiled, authored);
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(&program, Default::default());
    let reference = NormalizedReferenceInterpreter::new(&source, &program, Default::default());
    for declaration in authored {
        let (actual, expected) = vm.invoke_test(declaration, None, &control).unwrap();
        let (reference_actual, reference_expected) =
            reference.invoke_test(declaration, None, &control).unwrap();
        assert_eq!(actual.0, expected.0, "production {declaration:?}");
        assert_eq!(
            reference_actual.0, reference_expected.0,
            "reference {declaration:?}"
        );
        assert_eq!(actual.0, reference_actual.0, "differential {declaration:?}");
    }
}
