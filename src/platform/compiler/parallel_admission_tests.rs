//! Rehashed derived code cannot invent, erase or retarget structured children.
use super::*;
use crate::platform::diagnostic::DiagnosticClass;

const SOURCE: &str = r#"declarations.begin
(units (module create parallel-artifact
  (function create first (visibility private) (effect (task)) (returns I64)
    (body (i64 17)))
  (function create second (visibility private) (effect (task)) (returns I64)
    (body (i64 -29)))
  (function create joined (visibility public) (effect (task))
    (returns (record (left I64) (right I64)))
    (body (parallel (call first) (call second))))))
declarations.end
"#;

fn fixture() -> (LoadedArtifact, ObjectKey, CompilationUnit) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .unwrap();
    let loaded = artifact_for_source(&source);
    let owner = OwnerKey::Declaration(declaration_named(&source, "joined"));
    let (key, unit) = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap();
    (loaded, key, unit)
}

fn artifact_for_source(source: &crate::platform::kernel::KernelSnapshot) -> LoadedArtifact {
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&directory.path().join("parallel"), source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let artifact = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    load_artifact(&artifact.artifact.bytes).unwrap()
}

// Rebind logical source generations without changing canonical owners, compiler
// units or the current artifact generation. The neutral bundle writer does not
// invoke the strict loader, and every enclosing identity is recomputed.
fn rehash_logical_generation(
    loaded: &LoadedArtifact,
    package_generation: u16,
    source_generation: u16,
) -> Vec<u8> {
    use crate::platform::package_transport::PackageRevision;

    let mut objects = loaded.objects.clone();
    let mut manifest = loaded.manifest.clone();
    assert_eq!(manifest.packages.len(), 1);
    let package = &mut manifest.packages[0];
    let old_revision = ObjectKey::from_digest(
        ObjectDomain::PackageRevision,
        package.package_revision.bytes(),
    );
    let mut revision =
        PackageRevision::decode(&objects[&old_revision], package.package_revision).unwrap();
    assert!(revision.dependencies.is_empty());
    revision.graph_contract_version = package_generation;
    revision.revision.graph_contract_version = source_generation;
    revision.interface = crate::platform::package_interface::package_interface_digest_for_graph(
        package.package,
        package.interface_owners.content_root(),
        package_generation,
    )
    .unwrap();
    let (revision_digest, revision_bytes) = revision.encode().unwrap();
    objects.remove(&old_revision);
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::PackageRevision, revision_digest.bytes()),
        revision_bytes,
    );
    let old_compilation = package.compilation;
    let mut compilation =
        CompilationManifest::decode(&objects[&old_compilation.object_key()], old_compilation)
            .unwrap();
    compilation.revision = revision.revision.revision_id().unwrap();
    compilation.package_revision = revision_digest;
    compilation.package_interface = revision.interface;
    let (compilation_digest, compilation_bytes) = compilation.encode().unwrap();
    objects.remove(&old_compilation.object_key());
    objects.insert(compilation_digest.object_key(), compilation_bytes);
    package.package_revision = revision_digest;
    package.semantic_revision = compilation.revision;
    package.interface = revision.interface;
    package.compilation = compilation_digest;
    let (closure, count, bytes) = super::super::artifact::closure_facts(&objects).unwrap();
    manifest.closure = closure;
    manifest.object_count = count;
    manifest.object_bytes = bytes;
    nominal_session_tests::hostile_bundle(&manifest, &objects)
}

#[test]
fn parallel_artifact_rejects_rehashed_predecessor_logical_source() {
    let mut source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .unwrap();
    // Only Parallel needs Graph 21. Unchanged surrounding owners retain supported
    // Graph 20 encodings, so the attack cannot fail on an unrelated newer owner.
    for owner in source.owners.values_mut() {
        if !matches!(owner, OwnerRecord::Expression(expression)
            if matches!(expression.operation, ExpressionOperation::Parallel { .. }))
        {
            owner.set_encoding_for_edit(20);
        }
    }
    crate::platform::kernel::validate_full(&source).unwrap();
    let loaded = artifact_for_source(&source);
    load_artifact(&rehash_logical_generation(&loaded, 21, 21)).unwrap();
    source.root.graph_contract_version = 20;
    assert!(
        crate::platform::kernel::validate_full(&source)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "kernel_owner_graph_generation")
    );
    assert!(!crate::platform::kernel::memory_reference::accepts(&source));
    for package_generation in [20, 21] {
        let error = load_artifact(&rehash_logical_generation(&loaded, package_generation, 20))
            .map(|_| ())
            .expect_err("current derived contracts cannot authorize newer canonical meaning");
        assert_eq!(error.code, "kernel_owner_graph_generation", "{error:?}");
    }
}

#[test]
fn parallel_successor_rebuilds_supported_graph_20_meaning() {
    let input = SOURCE.replace(
        "(parallel (call first) (call second))",
        "(record structural (field left (call first)) (field right (call second)))",
    );
    let mut source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&input)
            .unwrap();
    source.root.graph_contract_version = 20;
    for owner in source.owners.values_mut() {
        owner.set_encoding_for_edit(20);
    }
    crate::platform::kernel::validate_full(&source).unwrap();
    let loaded = artifact_for_source(&source);
    assert_eq!(loaded.manifest.graph_contract_version, 21);
    // The transport producer can also remain at Graph 20 around old source.
    load_artifact(&rehash_logical_generation(&loaded, 20, 20)).unwrap();
}

#[test]
fn parallel_artifact_logical_source_generation_also_bounds_signature_types() {
    for (ty, use_mode, generation, expected) in [
        ("F64", "", 16, "artifact_f64_graph_generation"),
        (
            "(owned-product (field data ByteBuffer))",
            "(use consume)",
            18,
            "kernel_product_generation",
        ),
        (
            "(owned-choice (case data ByteBuffer))",
            "(use consume)",
            19,
            "kernel_choice_generation",
        ),
    ] {
        // A private, uncalled Local-only body has no new operation or exported
        // signature to provide an accidental substitute for complete type closure.
        let input = format!(
            "declarations.begin\n(units (module create signatures
              (function create relay (visibility private) (effect pure)
                (parameter create value (type {ty}) {use_mode})
                (returns {ty}) (body (local value)))))\ndeclarations.end"
        );
        let source =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&input)
                .unwrap();
        let loaded = artifact_for_source(&source);
        load_artifact(&rehash_logical_generation(&loaded, 21, 21)).unwrap();
        let error = load_artifact(&rehash_logical_generation(&loaded, 21, generation))
            .map(|_| ())
            .expect_err("current package/derived envelopes cannot upgrade source type permission");
        assert_eq!(error.code, expected, "{error:?}");
        assert_eq!(
            error.class,
            if generation == 16 {
                DiagnosticClass::Corrupt
            } else {
                DiagnosticClass::Semantic
            },
        );
    }
}

#[test]
fn parallel_artifact_rejects_rehashed_retargeting_and_erasure() {
    let (loaded, key, unit) = fixture();
    load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![])).unwrap();
    for attack in 0..3 {
        let mut changed = unit.clone();
        let CompilationPayload::Function { code, .. } = &mut changed.payload else {
            panic!("function");
        };
        let at = code
            .instructions
            .iter()
            .position(|i| matches!(i, CompiledInstruction::Parallel { .. }))
            .unwrap();
        let CompiledInstruction::Parallel {
            left,
            right,
            left_arguments: 0,
            right_arguments: 0,
        } = code.instructions[at]
        else {
            panic!("zero-argument pair");
        };
        code.instructions[at] = match attack {
            0 => CompiledInstruction::Parallel {
                left: right,
                right: left,
                left_arguments: 0,
                right_arguments: 0,
            },
            1 => CompiledInstruction::Parallel {
                left,
                right: left,
                left_arguments: 0,
                right_arguments: 0,
            },
            _ => CompiledInstruction::Unit,
        };
        let error = load_artifact(&effect_tests::replace_unit(&loaded, key, &changed, vec![]))
            .expect_err("rehashing cannot change canonical parallel control");
        assert!(
            matches!(
                error.code.as_str(),
                "artifact_compiled_control_meaning" | "artifact_nominal_instruction_meaning"
            ),
            "{error:?}"
        );
    }
}

#[test]
fn parallel_artifact_rejects_predecessor_unit_and_mislabeled_manifest() {
    let (loaded, key, mut unit) = fixture();
    unit.contract_version = 18;
    unit.bytecode_contract_version = 14;
    unit.graph_contract_version = 20;
    unit.key =
        CompilationUnitKey::derive_generation(&unit.source, unit.optimization, 18, 14, 20).unwrap();
    let error =
        load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![])).unwrap_err();
    assert_eq!(error.code, "compiler_unit_contract");
    let mut manifest = loaded.manifest.clone();
    manifest.contract_version = 25;
    assert_eq!(
        manifest.encode().unwrap_err().code,
        "artifact_manifest_contract"
    );
}
